use std::cmp::Ordering;
use std::default::Default;

use cursive::traits::*;
use cursive::Cursive;
use cursive::theme;
use crate::model::{MultiRepoHistory, Entry};

use crate::table_view::{TableView, TableViewItem};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum Column {
    CommitDateTime,
    Comitter,
    Repo,
    Summary
}

impl TableViewItem<Column> for Entry {
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

pub fn show(model: MultiRepoHistory) {

    let mut siv = Cursive::default();
    siv.load_toml(include_str!("../assets/style.toml")).unwrap();

    let mut table = TableView::<Entry, Column>::new()
        .column(Column::CommitDateTime, "Commit", |c| c.width(22))
        .column(Column::Repo, "Repo", |c|  c.width(model.max_width_repo).color(theme::ColorStyle::secondary()))
        .column(Column::Comitter, "Committer", |c| c.width(model.max_width_committer).color(theme::ColorStyle::tertiary()))
        .column(Column::Summary, "Summary", |c| c.color(theme::ColorStyle::tertiary()));

    table.set_items(model.commits);
    table.set_on_submit(|siv: &mut Cursive, row: usize, index: usize| {
        let value = siv
            .call_on_id("table", move |table: &mut TableView<Entry, Column>| {
                format!("{:?}", table.borrow_item(index).unwrap())
            })
            .unwrap();
    });

    siv.add_fullscreen_layer(table.full_screen());
    siv.add_global_callback('q', |s| s.quit());
    siv.run();
}
