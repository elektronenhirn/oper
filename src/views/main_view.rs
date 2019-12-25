use crate::model::{MultiRepoHistory, RepoCommit};
use crate::views::table_view::{TableView, TableViewItem};
use crate::styles::{GREEN, RED, WHITE};
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::traits::*;
use cursive::view::ViewWrapper;
use cursive::views::{Canvas, LinearLayout, ViewRef};
use cursive::Cursive;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

const COLUMN_WIDTH_COMMIT_DATE : usize = 22;
const COLUMN_WIDTH_REPO_NAME : usize = 15;
const COLUMN_WIDTH_COMITTER : usize = 17;
const COLUMN_WIDTH_SUBJECT : usize = 50;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum Column {
    CommitDateTime,
    Comitter,
    Repo,
    Summary,
}

impl TableViewItem<Column> for RepoCommit {
    fn to_column(&self, column: Column) -> String {
        match column {
            Column::CommitDateTime => self.time_as_str(),
            Column::Comitter => self.committer.clone(),
            Column::Repo => self.repo.description.clone(),
            Column::Summary => self.summary.clone(),
        }
    }

    fn cmp(&self, other: &Self, column: Column) -> Ordering
    where
        Self: Sized,
    {
        match column {
            Column::CommitDateTime => self.time_as_str().cmp(&other.time_as_str()),
            Column::Comitter => self.committer.cmp(&other.committer),
            Column::Repo => self.repo.description.cmp(&other.repo.description),
            Column::Summary => self.summary.cmp(&other.summary),
        }
    }
}

pub struct MainView {
    layout: LinearLayout,
    commit_bar_model: Rc<RefCell<String>>,
}

impl MainView {
    pub fn from(model: MultiRepoHistory) -> Self {
        let table = Self::new_table(model);
        let commit_bar_model = Rc::new(RefCell::new(String::from("")));
        let commit_bar = Self::new_commit_bar(commit_bar_model.clone());

        MainView {
            layout: LinearLayout::vertical()
                .child(table.with_id("table").full_screen())
                .child(commit_bar),
            commit_bar_model,
        }
    }

    pub fn set_on_select<F>(&mut self, cb: F)
    where
        F: Fn(&mut Cursive, usize, usize, &RepoCommit) + 'static,
    {
        let mut table: ViewRef<TableView<RepoCommit, Column>> =
            self.layout.find_id("table").unwrap();
        table.set_on_select(move |siv: &mut Cursive, row: usize, index: usize| {
            let entry = siv
                .call_on_id("table", move |table: &mut TableView<RepoCommit, Column>| {
                    table.borrow_item(index).unwrap().clone()
                })
                .unwrap();
            cb(siv, row, index, &entry)
        });
    }

    pub fn current_commit(&mut self) -> Option<RepoCommit> {
        let mut table: ViewRef<TableView<RepoCommit, Column>> =
            self.layout.find_id("table").unwrap();

        table.row().map_or(None, |row| {
            table
                .borrow_item(row)
                .map_or(None, |commit| Some(commit.clone()))
        })
    }

    fn new_table(model: MultiRepoHistory) -> TableView<RepoCommit, Column> {
        let mut table = TableView::<RepoCommit, Column>::new()
            .column(Column::CommitDateTime, "Commit", |c| c.width(COLUMN_WIDTH_COMMIT_DATE))
            .column(Column::Repo, "Repo", |c| {
                c.width(COLUMN_WIDTH_REPO_NAME).color(*RED)
            })
            .column(Column::Comitter, "Committer", |c| {
                c.width(COLUMN_WIDTH_COMITTER).color(*GREEN)
            })
            .column(Column::Summary, "Summary", |c| {
                c.width(COLUMN_WIDTH_SUBJECT).color(*WHITE)
            });
        table.set_items(model.commits);
        table.set_selected_row(0);

        table
    }

    fn new_commit_bar(model: Rc<RefCell<String>>) -> impl cursive::view::View {
        Canvas::new(model)
            .with_draw(|model, printer| {
                let style =
                    ColorStyle::new(Color::Dark(BaseColor::White), Color::Dark(BaseColor::Blue));
                printer.with_style(style, |p| {
                    let text = (*(*model).borrow()).clone();
                    p.print((0, 0), &text);
                    if p.size.x > text.len() {
                        p.print_hline((text.len(), 0), p.size.x - text.len(), " ");
                    }
                });
            })
            .with_required_size(|_model, req| cursive::Vec2::new(req.x, 1))
    }

    pub fn update_commit_bar(self: &mut Self, index: usize, size: usize, entry: &RepoCommit) {
        (*self.commit_bar_model).replace(format!(
            "Commit {} of {} - {}",
            index + 1,
            size,
            entry.repo.rel_path
        ));
    }
}

impl ViewWrapper for MainView {
    type V = LinearLayout;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        Some(f(&self.layout))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        Some(f(&mut self.layout))
    }
}
