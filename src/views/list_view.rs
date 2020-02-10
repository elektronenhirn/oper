// A customized version of https://github.com/BonsaiDen/cursive_table_view
// - Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
// - MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT) at your option.

// Changes:
//  - Removed Header and grid
//  - Removed columns and sorting
//  - Allowing different colors items

//! A basic list view implementation for [cursive](https://crates.io/crates/cursive).
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
extern crate cursive;

// STD Dependencies -----------------------------------------------------------
use std::cmp;
use std::rc::Rc;

// External Dependencies ------------------------------------------------------
use cursive::direction::Direction;
use cursive::event::{Callback, Event, EventResult, Key};
use cursive::theme;
use cursive::theme::{ColorStyle, Style};
use cursive::utils::span::{SpannedStr, SpannedString};
use cursive::vec::Vec2;
use cursive::view::{ScrollBase, View};
use cursive::With;
use cursive::{Cursive, Printer};

/// Callback taking as argument the row and the index of an element.
///
/// This is a private type to help readability.
type IndexCallback = Rc<dyn Fn(&mut Cursive, usize, usize)>;

/// View to select an SpnnedString among a list
pub struct ListView {
    enabled: bool,
    scrollbase: ScrollBase,
    last_size: Vec2,

    focus: usize,
    items: Vec<SpannedString<Style>>,
    rows_to_items: Vec<usize>,

    // TODO Pass drawing offsets into the handlers so a popup menu
    // can be created easily?
    on_submit: Option<IndexCallback>,
    on_select: Option<IndexCallback>,
}
impl Default for ListView {
    /// Creates a new empty `ListView` without any columns.
    ///
    /// See [`ListView::new()`].
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ListView {
    /// Creates a new empty `ListView` without any columns.
    ///
    /// A ListView should be accompanied by a enum of type `H` representing
    /// the table columns.
    pub fn new() -> Self {
        Self {
            enabled: true,
            scrollbase: ScrollBase::new(),
            last_size: Vec2::new(0, 0),

            focus: 0,
            items: Vec::new(),
            rows_to_items: Vec::new(),

            on_submit: None,
            on_select: None,
        }
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

    /// Sets a callback to be used when `<Enter>` is pressed while an item
    /// is selected.
    ///
    /// Both the currently selected row and the index of the corresponding item
    /// within the underlying storage vector will be given to the callback.
    ///
    /// # Example
    ///
    /// ```norun
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
    /// ```norun
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
    /// ```norun
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
    /// ```norun
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
        self.scrollbase.scroll_to(row_index);
    }

    /// Selects the row at the specified index.
    ///
    /// Chainable variant.
    pub fn selected_row(self, row_index: usize) -> Self {
        self.with(|t| t.set_selected_row(row_index))
    }

    /// Returns a immmutable reference to the item at the specified index
    /// within the underlying storage vector.
    pub fn borrow_item(&mut self, index: usize) -> Option<&SpannedString<Style>> {
        self.items.get(index)
    }

    /// Returns a mutable reference to the item at the specified index within
    /// the underlying storage vector.
    pub fn borrow_item_mut(&mut self, index: usize) -> Option<&mut SpannedString<Style>> {
        self.items.get_mut(index)
    }

    /// Returns a immmutable reference to the items contained within the table.
    pub fn borrow_items(&mut self) -> &Vec<SpannedString<Style>> {
        &self.items
    }

    /// Returns a mutable reference to the items contained within the table.
    ///
    /// Can be used to modify the items in place.
    pub fn borrow_items_mut(&mut self) -> &mut Vec<SpannedString<Style>> {
        &mut self.items
    }

    /// Returns the index of the currently selected item within the underlying
    /// storage vector.
    pub fn item(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.rows_to_items[self.focus])
        }
    }

    /// Selects the item at the specified index within the underlying storage
    /// vector.
    pub fn set_selected_item(&mut self, item_index: usize) {
        // TODO optimize the performance for very large item lists
        if item_index < self.items.len() {
            for (row, item) in self.rows_to_items.iter().enumerate() {
                if *item == item_index {
                    self.focus = row;
                    self.scrollbase.scroll_to(row);
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
    fn insert_item(&mut self, item: SpannedString<Style>) {
        self.items.push(item);
        self.rows_to_items.push(self.items.len() - 1);

        self.scrollbase
            .set_heights(self.last_size.y.saturating_sub(2), self.rows_to_items.len());
    }

    pub fn insert_string(&mut self, s: String) {
        for line in s.split('\n') {
            self.insert_item(SpannedString::<Style>::plain(line));
        }
    }

    pub fn insert_colorful_string(&mut self, s: String, c: ColorStyle) {
        for line in s.split('\n') {
            self.insert_item(SpannedString::styled(line, c));
        }
    }

    /// Removes the item at the specified index within the underlying storage
    /// vector and returns it.
    pub fn remove_item(&mut self, item_index: usize) -> Option<SpannedString<Style>> {
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

            // Update scroll height to prevent out of index drawing
            self.scrollbase
                .set_heights(self.last_size.y, self.rows_to_items.len());

            // Remove actual item from the underlying storage
            Some(self.items.remove(item_index))
        } else {
            None
        }
    }

    /// Removes all items from the underlying storage and returns them.
    pub fn take_items(&mut self) -> Vec<SpannedString<Style>> {
        self.scrollbase
            .set_heights(self.last_size.y.saturating_sub(2), 0);
        self.set_selected_row(0);
        self.rows_to_items.clear();
        self.items.drain(0..).collect()
    }
}

impl ListView {
    fn draw_item(&self, focused: bool, printer: &Printer, i: usize) {
        let item = &self.items[i];
        if focused {
            let item_without_color = SpannedString::<Style>::plain(item.source());
            printer.with_style(theme::ColorStyle::highlight(), |printer: &Printer| {
                printer.print_styled((0, 0), SpannedStr::from(&item_without_color));
            });
        } else {
            printer.print_styled((0, 0), SpannedStr::from(item));
        }
    }

    fn focus_up(&mut self, n: usize) {
        self.focus -= cmp::min(self.focus, n);
    }

    fn focus_down(&mut self, n: usize) {
        self.focus = cmp::min(self.focus + n, self.items.len() - 1);
    }
}

impl View for ListView {
    fn draw(&self, printer: &Printer) {
        let printer = &printer.focused(true);
        self.scrollbase.draw(printer, |printer, i| {
            if i < self.items.len() {
                self.draw_item(self.focus == i, printer, i);
            }
        });
    }

    fn layout(&mut self, size: Vec2) {
        if size == self.last_size {
            return;
        }

        let item_count = self.items.len();

        self.scrollbase.set_heights(size.y, item_count);
        self.last_size = size;
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        let last_focus = self.focus;
        match event {
            Event::Key(Key::Up) if self.focus > 0 => {
                self.focus_up(1);
            }
            Event::Key(Key::Down) if self.focus + 1 < self.items.len() => {
                self.focus_down(1);
            }
            Event::Key(Key::PageUp) => {
                self.focus_up(10);
            }
            Event::Key(Key::PageDown) => {
                self.focus_down(10);
            }
            Event::Key(Key::Home) => {
                self.focus = 0;
            }
            Event::Key(Key::End) => {
                self.focus = self.items.len() - 1;
            }
            Event::Key(Key::Enter) => {
                if !self.is_empty() && self.on_submit.is_some() {
                    let cb = self.on_submit.clone().unwrap();
                    let row = self.row().unwrap();
                    let index = self.item().unwrap();
                    return EventResult::Consumed(Some(Callback::from_fn(move |s| {
                        cb(s, row, index)
                    })));
                }
            }
            _ => return EventResult::Ignored,
        }

        let focus = self.focus;
        self.scrollbase.scroll_to(focus);

        if !self.is_empty() && last_focus != focus {
            let row = self.row().unwrap();
            let index = self.item().unwrap();
            EventResult::Consumed(
                self.on_select
                    .clone()
                    .map(|cb| Callback::from_fn(move |s| cb(s, row, index))),
            )
        } else {
            EventResult::Ignored
        }
    }
}
