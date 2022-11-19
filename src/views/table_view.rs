// A customized version of https://github.com/BonsaiDen/cursive_table_view
// - Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
// - MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT) at your option.

// Changes:
//  - Added support for column-specific colors
//  - Removed sorting of columns

#![deny(
    missing_docs,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

// Crate Dependencies ---------------------------------------------------------
use cursive;

// STD Dependencies -----------------------------------------------------------
use std::cmp::{self, Ordering};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

// External Dependencies ------------------------------------------------------
use cursive::{
    align::HAlign,
    direction::Direction,
    event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent},
    theme,
    vec::Vec2,
    view::{scroll, CannotFocus, View},
    Cursive, Printer, Rect, With,
};

/// A trait for displaying and sorting items inside a
/// [`TableView`](struct.TableView.html).
pub trait TableViewItem<H>: Clone + Sized
where
    H: Eq + Hash + Copy + Clone + 'static,
{
    /// Method returning a string representation of the item for the
    /// specified column from type `H`.
    fn to_column(&self, column: H) -> String;

    /// Method comparing two items via their specified column from type `H`.
    fn cmp(&self, other: &Self, column: H) -> Ordering
    where
        Self: Sized;
}

/// Callback used when a column is sorted.
///
/// It takes the column and the ordering as input.
///
/// This is a private type to help readability.
type OnSortCallback<H> = Rc<dyn Fn(&mut Cursive, H, Ordering)>;

/// Callback taking as argument the row and the index of an element.
///
/// This is a private type to help readability.
type IndexCallback = Rc<dyn Fn(&mut Cursive, usize, usize)>;

/// View to select an item among a list, supporting multiple columns for sorting.
///
/// # Examples
///
/// ```rust
/// # extern crate cursive;
/// # extern crate cursive_table_view;
/// # use std::cmp::Ordering;
/// # use cursive_table_view::{TableView, TableViewItem};
/// # use cursive::align::HAlign;
/// # fn main() {
/// // Provide a type for the table's columns
/// #[derive(Copy, Clone, PartialEq, Eq, Hash)]
/// enum BasicColumn {
///     Name,
///     Count,
///     Rate
/// }
///
/// // Define the item type
/// #[derive(Clone, Debug)]
/// struct Foo {
///     name: String,
///     count: usize,
///     rate: usize
/// }
///
/// impl TableViewItem<BasicColumn> for Foo {
///
///     fn to_column(&self, column: BasicColumn) -> String {
///         match column {
///             BasicColumn::Name => self.name.to_string(),
///             BasicColumn::Count => format!("{}", self.count),
///             BasicColumn::Rate => format!("{}", self.rate)
///         }
///     }
///
///     fn cmp(&self, other: &Self, column: BasicColumn) -> Ordering where Self: Sized {
///         match column {
///             BasicColumn::Name => self.name.cmp(&other.name),
///             BasicColumn::Count => self.count.cmp(&other.count),
///             BasicColumn::Rate => self.rate.cmp(&other.rate)
///         }
///     }
///
/// }
///
/// // Configure the actual table
/// let table = TableView::<Foo, BasicColumn>::new()
///                      .column(BasicColumn::Name, "Name", |c| c.width(20))
///                      .column(BasicColumn::Count, "Count", |c| c.align(HAlign::Center))
///                      .column(BasicColumn::Rate, "Rate", |c| {
///                          c.ordering(Ordering::Greater).align(HAlign::Right).width(20)
///                      })
///                      .default_column(BasicColumn::Name);
/// # }
/// ```
pub struct TableView<T, H> {
    enabled: bool,
    scroll_core: scroll::Core,
    needs_relayout: bool,

    column_select: bool,
    columns: Vec<TableColumn<H>>,
    column_indicies: HashMap<H, usize>,

    focus: usize,
    items: Vec<T>,
    rows_to_items: Vec<usize>,

    on_sort: Option<OnSortCallback<H>>,
    // TODO Pass drawing offsets into the handlers so a popup menu
    // can be created easily?
    on_submit: Option<IndexCallback>,
    on_select: Option<IndexCallback>,
}

cursive::impl_scroller!(TableView < T, H > ::scroll_core);

#[allow(dead_code)]
impl<T, H> Default for TableView<T, H>
where
    T: TableViewItem<H> + PartialEq,
    H: Eq + Hash + Copy + Clone + 'static,
{
    /// Creates a new empty `TableView` without any columns.
    ///
    /// See [`TableView::new()`].
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl<T, H> TableView<T, H>
where
    T: TableViewItem<H> + PartialEq,
    H: Eq + Hash + Copy + Clone + 'static,
{
    /// Sets the contained items of the table.
    ///
    /// The currently active sort order is preserved and will be applied to all
    /// items.
    ///
    /// Compared to `set_items`, the current selection will be preserved.
    /// (But this is only available for `T: PartialEq`.)
    pub fn set_items_stable(&mut self, items: Vec<T>) {
        // Preserve selection
        let new_location = self
            .item()
            .and_then(|old_item| {
                let old_item = &self.items[old_item];
                items.iter().position(|new| new == old_item)
            })
            .unwrap_or(0);

        self.set_items_and_focus(items, new_location);
    }
}

#[allow(dead_code)]
impl<T, H> TableView<T, H>
where
    T: TableViewItem<H>,
    H: Eq + Hash + Copy + Clone + 'static,
{
    /// Creates a new empty `TableView` without any columns.
    ///
    /// A TableView should be accompanied by a enum of type `H` representing
    /// the table columns.
    pub fn new() -> Self {
        Self {
            enabled: true,
            scroll_core: scroll::Core::new(),
            needs_relayout: true,

            column_select: false,
            columns: Vec::new(),
            column_indicies: HashMap::new(),

            focus: 0,
            items: Vec::new(),
            rows_to_items: Vec::new(),

            on_sort: None,
            on_submit: None,
            on_select: None,
        }
    }

    /// Adds a column for the specified table colum from type `H` along with
    /// a title for its visual display.
    ///
    /// The provided callback can be used to further configure the
    /// created [`TableColumn`](struct.TableColumn.html).
    pub fn column<S: Into<String>, C: FnOnce(TableColumn<H>) -> TableColumn<H>>(
        mut self,
        column: H,
        title: S,
        callback: C,
    ) -> Self {
        self.add_column(column, title, callback);
        self
    }

    /// Adds a column for the specified table colum from type `H` along with
    /// a title for its visual display.
    ///
    /// The provided callback can be used to further configure the
    /// created [`TableColumn`](struct.TableColumn.html).
    pub fn add_column<S: Into<String>, C: FnOnce(TableColumn<H>) -> TableColumn<H>>(
        &mut self,
        column: H,
        title: S,
        callback: C,
    ) {
        self.insert_column(self.columns.len(), column, title, callback);
    }

    /// Remove a column.
    pub fn remove_column(&mut self, i: usize) {
        // Update the existing indices
        for column in &self.columns[i + 1..] {
            *self.column_indicies.get_mut(&column.column).unwrap() -= 1;
        }

        let column = self.columns.remove(i);
        self.column_indicies.remove(&column.column);
        self.needs_relayout = true;
    }

    /// Adds a column for the specified table colum from type `H` along with
    /// a title for its visual display.
    ///
    /// The provided callback can be used to further configure the
    /// created [`TableColumn`](struct.TableColumn.html).
    pub fn insert_column<S: Into<String>, C: FnOnce(TableColumn<H>) -> TableColumn<H>>(
        &mut self,
        i: usize,
        column: H,
        title: S,
        callback: C,
    ) {
        // Update all existing indices
        for column in &self.columns[i..] {
            *self.column_indicies.get_mut(&column.column).unwrap() += 1;
        }

        self.column_indicies.insert(column, i);
        self.columns
            .insert(i, callback(TableColumn::new(column, title.into())));

        self.needs_relayout = true;
    }

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Enable or disable this view.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets a callback to be used when a selected column is sorted by
    /// pressing `<Enter>`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// table.set_on_sort(|siv: &mut Cursive, column: BasicColumn, order: Ordering| {
    ///
    /// });
    /// ```
    pub fn set_on_sort<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, H, Ordering) + 'static,
    {
        self.on_sort = Some(Rc::new(move |s, h, o| cb(s, h, o)));
    }

    /// Sets a callback to be used when a selected column is sorted by
    /// pressing `<Enter>`.
    ///
    /// Chainable variant.
    ///
    /// # Example
    ///
    /// ```ignore
    /// table.on_sort(|siv: &mut Cursive, column: BasicColumn, order: Ordering| {
    ///
    /// });
    /// ```
    pub fn on_sort<F>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, H, Ordering) + 'static,
    {
        self.with(|t| t.set_on_sort(cb))
    }

    /// Sets a callback to be used when `<Enter>` is pressed while an item
    /// is selected.
    ///
    /// Both the currently selected row and the index of the corresponding item
    /// within the underlying storage vector will be given to the callback.
    ///
    /// # Example
    ///
    /// ```ignore
    /// table.set_on_submit(|siv: &mut Cursive, row: usize, index: usize| {
    ///
    /// });
    /// ```
    pub fn set_on_submit<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, usize, usize) + 'static,
    {
        self.on_submit = Some(Rc::new(move |s, row, index| cb(s, row, index)));
    }

    /// Sets a callback to be used when `<Enter>` is pressed while an item
    /// is selected.
    ///
    /// Both the currently selected row and the index of the corresponding item
    /// within the underlying storage vector will be given to the callback.
    ///
    /// Chainable variant.
    ///
    /// # Example
    ///
    /// ```ignore
    /// table.on_submit(|siv: &mut Cursive, row: usize, index: usize| {
    ///
    /// });
    /// ```
    pub fn on_submit<F>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, usize, usize) + 'static,
    {
        self.with(|t| t.set_on_submit(cb))
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// Both the currently selected row and the index of the corresponding item
    /// within the underlying storage vector will be given to the callback.
    ///
    /// # Example
    ///
    /// ```ignore
    /// table.set_on_select(|siv: &mut Cursive, row: usize, index: usize| {
    ///
    /// });
    /// ```
    pub fn set_on_select<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, usize, usize) + 'static,
    {
        self.on_select = Some(Rc::new(move |s, row, index| cb(s, row, index)));
    }

    /// Sets a callback to be used when an item is selected.
    ///
    /// Both the currently selected row and the index of the corresponding item
    /// within the underlying storage vector will be given to the callback.
    ///
    /// Chainable variant.
    ///
    /// # Example
    ///
    /// ```ignore
    /// table.on_select(|siv: &mut Cursive, row: usize, index: usize| {
    ///
    /// });
    /// ```
    pub fn on_select<F>(self, cb: F) -> Self
    where
        F: Fn(&mut Cursive, usize, usize) + 'static,
    {
        self.with(|t| t.set_on_select(cb))
    }

    /// Removes all items from this view.
    pub fn clear(&mut self) {
        self.items.clear();
        self.rows_to_items.clear();
        self.focus = 0;
        self.needs_relayout = true;
    }

    /// Returns the number of items in this table.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if this table has no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the index of the currently selected table row.
    pub fn row(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.focus)
        }
    }

    /// Selects the row at the specified index.
    pub fn set_selected_row(&mut self, row_index: usize) {
        self.focus = row_index;
        self.scroll_core.scroll_to_y(row_index);
    }

    /// Selects the row at the specified index.
    ///
    /// Chainable variant.
    pub fn selected_row(self, row_index: usize) -> Self {
        self.with(|t| t.set_selected_row(row_index))
    }

    /// Sets the contained items of the table.
    ///
    /// The currently active sort order is preserved and will be applied to all
    /// items.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.set_items_and_focus(items, 0);
    }

    fn set_items_and_focus(&mut self, items: Vec<T>, new_location: usize) {
        self.items = items;
        self.rows_to_items = Vec::with_capacity(self.items.len());

        for i in 0..self.items.len() {
            self.rows_to_items.push(i);
        }

        self.set_selected_item(new_location);
        self.needs_relayout = true;
    }

    /// Sets the contained items of the table.
    ///
    /// The order of the items will be preserved even when the table is sorted.
    ///
    /// Chainable variant.
    pub fn items(self, items: Vec<T>) -> Self {
        self.with(|t| t.set_items(items))
    }

    /// Returns a immmutable reference to the item at the specified index
    /// within the underlying storage vector.
    pub fn borrow_item(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Returns a mutable reference to the item at the specified index within
    /// the underlying storage vector.
    pub fn borrow_item_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    /// Returns a immmutable reference to the items contained within the table.
    pub fn borrow_items(&mut self) -> &[T] {
        &self.items
    }

    /// Returns a mutable reference to the items contained within the table.
    ///
    /// Can be used to modify the items in place.
    pub fn borrow_items_mut(&mut self) -> &mut [T] {
        self.needs_relayout = true;
        &mut self.items
    }

    /// Returns the index of the currently selected item within the underlying
    /// storage vector.
    pub fn item(&self) -> Option<usize> {
        self.rows_to_items.get(self.focus).copied()
    }

    /// Selects the item at the specified index within the underlying storage
    /// vector.
    pub fn set_selected_item(&mut self, item_index: usize) {
        // TODO optimize the performance for very large item lists
        if item_index < self.items.len() {
            for (row, item) in self.rows_to_items.iter().enumerate() {
                if *item == item_index {
                    self.focus = row;
                    self.scroll_core.scroll_to_y(row);
                    break;
                }
            }
        }
    }

    /// Selects the item at the specified index within the underlying storage
    /// vector.
    ///
    /// Chainable variant.
    pub fn selected_item(self, item_index: usize) -> Self {
        self.with(|t| t.set_selected_item(item_index))
    }

    /// Inserts a new item into the table.
    ///
    /// The currently active sort order is preserved and will be applied to the
    /// newly inserted item.
    ///
    /// If no sort option is set, the item will be added to the end of the table.
    pub fn insert_item(&mut self, item: T) {
        self.insert_item_at(self.items.len(), item);
    }

    /// Inserts a new item into the table.
    ///
    /// The currently active sort order is preserved and will be applied to the
    /// newly inserted item.
    ///
    /// If no sort option is set, the item will be inserted at the given index.
    ///
    /// # Panics
    ///
    /// If `index > self.len()`.
    pub fn insert_item_at(&mut self, index: usize, item: T) {
        self.items.push(item);

        // Here we know self.items.len() > 0
        self.rows_to_items.insert(index, self.items.len() - 1);

        self.needs_relayout = true;
    }

    /// Removes the item at the specified index within the underlying storage
    /// vector and returns it.
    pub fn remove_item(&mut self, item_index: usize) -> Option<T> {
        if item_index < self.items.len() {
            // Move the selection if the currently selected item gets removed
            if let Some(selected_index) = self.item() {
                if selected_index == item_index {
                    self.focus_up(1);
                }
            }

            // Remove the sorted reference to the item
            self.rows_to_items.retain(|i| *i != item_index);

            // Adjust remaining references
            for ref_index in &mut self.rows_to_items {
                if *ref_index > item_index {
                    *ref_index -= 1;
                }
            }
            self.needs_relayout = true;

            // Remove actual item from the underlying storage
            Some(self.items.remove(item_index))
        } else {
            None
        }
    }

    /// Removes all items from the underlying storage and returns them.
    pub fn take_items(&mut self) -> Vec<T> {
        self.set_selected_row(0);
        self.rows_to_items.clear();
        self.needs_relayout = true;
        self.items.drain(0..).collect()
    }
}

impl<T, H> TableView<T, H>
where
    T: TableViewItem<H>,
    H: Eq + Hash + Copy + Clone + 'static,
{
    fn draw_columns<C: Fn(&Printer, &TableColumn<H>)>(
        &self,
        printer: &Printer,
        sep: &str,
        callback: C,
    ) {
        let mut column_offset = 0;
        let column_count = self.columns.len();
        for (index, column) in self.columns.iter().enumerate() {
            let printer = &printer.offset((column_offset, 0)).focused(true);

            callback(printer, column);

            if 1 + index < column_count {
                printer.print((column.width + 1, 0), sep);
            }

            column_offset += column.width + 3;
        }
    }

    fn draw_item(&self, focused: bool, printer: &Printer, i: usize) {
        self.draw_columns(printer, "┆ ", |printer, column| {
            let value = self.items[self.rows_to_items[i]].to_column(column.column);
            column.draw_row(focused, printer, value.as_str());
        });
    }

    fn on_focus_change(&self) -> EventResult {
        let row = self.row().unwrap();
        let index = self.item().unwrap();
        EventResult::Consumed(
            self.on_select
                .clone()
                .map(|cb| Callback::from_fn(move |s| cb(s, row, index))),
        )
    }

    fn focus_up(&mut self, n: usize) {
        self.focus -= cmp::min(self.focus, n);
    }

    fn focus_down(&mut self, n: usize) {
        self.focus = cmp::min(self.focus + n, self.items.len().saturating_sub(1));
    }

    fn active_column(&self) -> usize {
        self.columns.iter().position(|c| c.selected).unwrap_or(0)
    }

    fn column_cancel(&mut self) {
        self.column_select = false;
    }

    fn column_for_x(&self, mut x: usize) -> Option<usize> {
        for (i, col) in self.columns.iter().enumerate() {
            x = match x.checked_sub(col.width) {
                None => return Some(i),
                Some(x) => x.checked_sub(3)?,
            };
        }

        None
    }

    fn draw_content(&self, printer: &Printer) {
        for i in 0..self.rows_to_items.len() {
            let printer = printer.offset((0, i));
            let color = if i == self.focus && self.enabled {
                if !self.column_select && self.enabled && printer.focused {
                    theme::ColorStyle::highlight()
                } else {
                    theme::ColorStyle::highlight_inactive()
                }
            } else {
                theme::ColorStyle::primary()
            };

            if i < self.items.len() {
                printer.with_color(color, |printer| {
                    self.draw_item(i == self.focus, printer, i);
                });
            }
        }
    }

    fn layout_content(&mut self, size: Vec2) {
        let column_count = self.columns.len();

        // Split up all columns into sized / unsized groups
        let (mut sized, mut usized): (Vec<&mut TableColumn<H>>, Vec<&mut TableColumn<H>>) = self
            .columns
            .iter_mut()
            .partition(|c| c.requested_width.is_some());

        // Subtract one for the seperators between our columns (that's column_count - 1)
        let available_width = size.x.saturating_sub(column_count.saturating_sub(1) * 3);

        // Calculate widths for all requested columns
        let mut remaining_width = available_width;
        for column in &mut sized {
            column.width = match *column.requested_width.as_ref().unwrap() {
                TableColumnWidth::Percent(width) => cmp::min(
                    (size.x as f32 / 100.0 * width as f32).ceil() as usize,
                    remaining_width,
                ),
                TableColumnWidth::Absolute(width) => width,
            };
            remaining_width = remaining_width.saturating_sub(column.width);
        }

        // Spread the remaining with across the unsized columns
        let remaining_columns = usized.len();
        for column in &mut usized {
            column.width = (remaining_width as f32 / remaining_columns as f32).floor() as usize;
        }

        self.needs_relayout = false;
    }

    fn content_required_size(&mut self, req: Vec2) -> Vec2 {
        Vec2::new(req.x, self.rows_to_items.len())
    }

    fn on_inner_event(&mut self, event: Event) -> EventResult {
        let last_focus = self.focus;
        match event {
            Event::Key(Key::Right) => {
                return EventResult::Ignored;
            }
            Event::Key(Key::Left) => {
                return EventResult::Ignored;
            }
            Event::Key(Key::Up) if self.focus > 0 || self.column_select => {
                if self.column_select {
                    self.column_cancel();
                } else {
                    self.focus_up(1);
                }
            }
            Event::Key(Key::Down) if self.focus + 1 < self.items.len() || self.column_select => {
                if self.column_select {
                    self.column_cancel();
                } else {
                    self.focus_down(1);
                }
            }
            Event::Key(Key::PageUp) => {
                self.column_cancel();
                self.focus_up(10);
            }
            Event::Key(Key::PageDown) => {
                self.column_cancel();
                self.focus_down(10);
            }
            Event::Key(Key::Home) => {
                self.column_cancel();
                self.focus = 0;
            }
            Event::Key(Key::End) => {
                self.column_cancel();
                self.focus = self.items.len().saturating_sub(1);
            }
            Event::Key(Key::Enter) => {
                if self.column_select {
                    return EventResult::Ignored;
                } else if !self.is_empty() && self.on_submit.is_some() {
                    return self.on_submit_event();
                }
            }
            Event::Mouse {
                position,
                offset,
                event: MouseEvent::Press(MouseButton::Left),
            } if !self.is_empty()
                && position
                    .checked_sub(offset)
                    .map_or(false, |p| p.y == self.focus) =>
            {
                self.column_cancel();
                return self.on_submit_event();
            }
            Event::Mouse {
                position,
                offset,
                event: MouseEvent::Press(_),
            } if !self.is_empty() => match position.checked_sub(offset) {
                Some(position) if position.y < self.rows_to_items.len() => {
                    self.column_cancel();
                    self.focus = position.y;
                }
                _ => return EventResult::Ignored,
            },
            _ => return EventResult::Ignored,
        }

        let focus = self.focus;

        if self.column_select {
            EventResult::Consumed(None)
        } else if !self.is_empty() && last_focus != focus {
            self.on_focus_change()
        } else {
            EventResult::Ignored
        }
    }

    fn inner_important_area(&self, size: Vec2) -> Rect {
        Rect::from_size((0, self.focus), (size.x, 1))
    }

    fn on_submit_event(&mut self) -> EventResult {
        if let Some(ref cb) = &self.on_submit {
            let cb = Rc::clone(cb);
            let row = self.row().unwrap();
            let index = self.item().unwrap();
            return EventResult::Consumed(Some(Callback::from_fn(move |s| cb(s, row, index))));
        }
        EventResult::Ignored
    }
}

impl<T, H> View for TableView<T, H>
where
    T: TableViewItem<H> + 'static,
    H: Eq + Hash + Copy + Clone + 'static,
{
    fn draw(&self, printer: &Printer) {
        self.draw_columns(printer, "╷ ", |printer, column| {
            let color = if self.enabled && column.selected {
                if self.column_select && column.selected && self.enabled && printer.focused {
                    theme::ColorStyle::highlight()
                } else {
                    theme::ColorStyle::highlight_inactive()
                }
            } else {
                theme::ColorStyle::primary()
            };

            printer.with_color(color, |printer| {
                column.draw_header(printer);
            });
        });

        self.draw_columns(
            &printer.offset((0, 1)).focused(true),
            "┴─",
            |printer, column| {
                printer.print_hline((0, 0), column.width + 1, "─");
            },
        );

        // Extend the vertical bars to the end of the view
        for y in 2..printer.size.y {
            self.draw_columns(&printer.offset((0, y)), "┆ ", |_, _| ());
        }

        let printer = &printer.offset((0, 2)).focused(true);
        scroll::draw(self, printer, Self::draw_content);
    }

    fn layout(&mut self, size: Vec2) {
        scroll::layout(
            self,
            size.saturating_sub((0, 2)),
            self.needs_relayout,
            Self::layout_content,
            Self::content_required_size,
        );
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        self.enabled.then(EventResult::consumed).ok_or(CannotFocus)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::Mouse {
                position,
                offset,
                event: MouseEvent::Press(MouseButton::Left),
            } if position.checked_sub(offset).map_or(false, |p| p.y == 0) => {
                if let Some(position) = position.checked_sub(offset) {
                    if let Some(col) = self.column_for_x(position.x) {
                        if self.column_select && self.columns[col].selected {
                            return EventResult::Ignored;
                        } else {
                            let active = self.active_column();
                            self.columns[active].selected = false;
                            self.columns[col].selected = true;
                            self.column_select = true;
                        }
                    }
                }
                EventResult::Ignored
            }
            event => scroll::on_event(
                self,
                event.relativized((0, 2)),
                Self::on_inner_event,
                Self::inner_important_area,
            ),
        }
    }

    fn important_area(&self, size: Vec2) -> Rect {
        self.inner_important_area(size.saturating_sub((0, 2))) + (0, 2)
    }
}

/// A type used for the construction of columns in a
/// [`TableView`](struct.TableView.html).
#[allow(dead_code)]
pub struct TableColumn<H> {
    column: H,
    title: String,
    selected: bool,
    alignment: HAlign,
    width: usize,
    requested_width: Option<TableColumnWidth>,
    color: theme::ColorStyle,
}

#[allow(dead_code)]
enum TableColumnWidth {
    Percent(usize),
    Absolute(usize),
}

#[allow(dead_code)]
impl<H: Copy + Clone + 'static> TableColumn<H> {
    /// Sets the horizontal text alignment of the column.
    pub fn align(mut self, alignment: HAlign) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets how many characters of width this column will try to occupy.
    pub fn width(mut self, width: usize) -> Self {
        self.requested_width = Some(TableColumnWidth::Absolute(width));
        self
    }

    /// Sets what percentage of the width of the entire table this column will
    /// try to occupy.
    pub fn width_percent(mut self, width: usize) -> Self {
        self.requested_width = Some(TableColumnWidth::Percent(width));
        self
    }

    pub fn color(mut self, color: theme::ColorStyle) -> Self {
        self.color = color;
        self
    }

    fn new(column: H, title: String) -> Self {
        Self {
            column,
            title,
            selected: false,
            alignment: HAlign::Left,
            width: 0,
            requested_width: None,
            color: theme::ColorStyle::primary(),
        }
    }

    fn draw_header(&self, printer: &Printer) {
        let header = match self.alignment {
            HAlign::Left => format!("{:<width$}", self.title, width = self.width),
            HAlign::Right => format!("{:>width$}", self.title, width = self.width),
            HAlign::Center => format!("{:^width$}", self.title, width = self.width),
        };

        printer.print((0, 0), header.as_str());
    }

    fn draw_row(&self, focused: bool, printer: &Printer, value: &str) {
        let value = match self.alignment {
            HAlign::Left => format!("{:<width$} ", value, width = self.width),
            HAlign::Right => format!("{:>width$} ", value, width = self.width),
            HAlign::Center => format!("{:^width$} ", value, width = self.width),
        };

        printer.with_color(
            if focused {
                theme::ColorStyle::highlight()
            } else {
                self.color
            },
            |printer| {
                printer.print((0, 0), value.as_str());
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    enum SimpleColumn {
        Name,
    }

    #[allow(dead_code)]
    impl SimpleColumn {
        fn as_str(&self) -> &str {
            match *self {
                SimpleColumn::Name => "Name",
            }
        }
    }

    #[derive(Clone, Debug)]
    struct SimpleItem {
        name: String,
    }

    impl TableViewItem<SimpleColumn> for SimpleItem {
        fn to_column(&self, column: SimpleColumn) -> String {
            match column {
                SimpleColumn::Name => self.name.to_string(),
            }
        }

        fn cmp(&self, other: &Self, column: SimpleColumn) -> Ordering
        where
            Self: Sized,
        {
            match column {
                SimpleColumn::Name => self.name.cmp(&other.name),
            }
        }
    }

    fn setup_test_table() -> TableView<SimpleItem, SimpleColumn> {
        TableView::<SimpleItem, SimpleColumn>::new()
            .column(SimpleColumn::Name, "Name", |c| c.width_percent(20))
    }

    #[test]
    fn should_insert_into_existing_table() {
        let mut simple_table = setup_test_table();

        let mut simple_items = Vec::new();

        for i in 1..=10 {
            simple_items.push(SimpleItem {
                name: format!("{} - Name", i),
            });
        }

        // Insert First Batch of Items
        simple_table.set_items(simple_items);

        // Test for Additional item insertion
        simple_table.insert_item(SimpleItem {
            name: format!("{} Name", 11),
        });

        assert!(simple_table.len() == 11);
    }

    #[test]
    fn should_insert_into_empty_table() {
        let mut simple_table = setup_test_table();

        // Test for First item insertion
        simple_table.insert_item(SimpleItem {
            name: format!("{} Name", 1),
        });

        assert!(simple_table.len() == 1);
    }
}
