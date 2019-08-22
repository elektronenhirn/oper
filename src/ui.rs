use crate::model::{Entry, MultiRepoHistory};
use crate::table_view::{TableView, TableViewItem};
use cursive::theme::{BaseColor, Color, ColorStyle, Style};
use cursive::traits::*;
use cursive::utils::span::SpannedString;
use cursive::views::{Canvas, LinearLayout};
use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::default::Default;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum Column {
    CommitDateTime,
    Comitter,
    Repo,
    Summary,
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

fn build_commit_bar(model: Rc<RefCell<String>>) -> impl cursive::view::View {
    Canvas::new(model)
        .with_draw(|model, printer| {
            let style =
                ColorStyle::new(Color::Dark(BaseColor::White), Color::Dark(BaseColor::Blue));
            printer.with_style(style, |p| {
                let text = (*(*model).borrow()).clone();
                p.print((0, 0), &text);
                p.print_hline((text.len(), 0), p.size.x - text.len(), " ");
            });
        })
        .with_required_size(|_model, req| cursive::Vec2::new(req.x, 1))
}

fn build_status_bar(model: Rc<String>) -> impl cursive::view::View {
    Canvas::new(model)
        .with_draw(|model, printer| {
            printer.with_style(ColorStyle::tertiary(), |p| p.print((0, 0), &model))
        })
        .with_required_size(|_model, req| cursive::Vec2::new(req.x, 1))
}

fn update_commit_bar(
    commit_bar_model: &Rc<RefCell<String>>,
    index: usize,
    size: usize,
    entry: &Entry,
) {
    (*commit_bar_model).replace(format!(
        "Commit {} of {} - {}",
        index + 1,
        size,
        entry.repo.rel_path
    ));
}

#[rustfmt::skip]
fn build_commit_view(entry: &Entry) -> TextView{
    let mut text = SpannedString::<Style>::plain("");

    text.append_styled(format!("Repo:       {}\n", entry.repo.rel_path), ColorStyle::primary() );
    text.append_styled(format!("Id:         {}\n", entry.commit_id),     ColorStyle::primary());
    text.append_styled(format!("Author:     {}\n", entry.author),        ColorStyle::tertiary() );
    text.append_styled(format!("Commit:     {}\n", entry.committer),     ColorStyle::tertiary() );
    text.append_styled(format!("CommitDate: {}\n", entry.time_as_str()), ColorStyle::secondary() );
    text.append("\n");

    text.append(&entry.message);

    TextView::new(text)
}

pub fn show(model: MultiRepoHistory) {
    let commit_bar = Rc::new(RefCell::new(String::from("")));
    let commit_bar_copy = commit_bar.clone();
    let status_bar = Rc::new(format!(
        "Found {} commits across {} repositories",
        model.commits.len(),
        model.repos.len()
    ));
    let commits = model.commits.len();
    if commits > 0 {
        update_commit_bar(&commit_bar, 0, commits, &model.commits[0]);
    }

    let mut siv = Cursive::default();
    siv.load_toml(include_str!("../assets/style.toml")).unwrap();

    let mut table = TableView::<Entry, Column>::new()
        .column(Column::CommitDateTime, "Commit", |c| c.width(22))
        .column(Column::Repo, "Repo", |c| {
            c.width(model.max_width_repo).color(ColorStyle::secondary())
        })
        .column(Column::Comitter, "Committer", |c| {
            c.width(model.max_width_committer)
                .color(ColorStyle::tertiary())
        })
        .column(Column::Summary, "Summary", |c| {
            c.color(ColorStyle::tertiary())
        });
    table.set_items(model.commits);
    table.set_on_select(move |siv: &mut Cursive, _row: usize, index: usize| {
        let entry = siv
            .call_on_id("table", move |table: &mut TableView<Entry, Column>| {
                table.borrow_item(index).unwrap().clone()
            })
            .unwrap();
        update_commit_bar(&commit_bar, index, commits, &entry);
    });

    table.set_on_submit(|siv: &mut Cursive, _row: usize, index: usize| {
        let entry = siv
            .call_on_id("table", move |table: &mut TableView<Entry, Column>| {
                table.borrow_item(index).unwrap().clone()
            })
            .unwrap();

        siv.add_layer(
            Dialog::around(build_commit_view(&entry)).button("Ok", move |s| {
                s.pop_layer();
            }),
        );
    });

    table.set_selected_row(0);
    let layout = LinearLayout::vertical()
        .child(table.with_id("table").full_screen())
        .child(build_commit_bar(commit_bar_copy))
        .child(build_status_bar(status_bar));
    siv.add_layer(layout);
    siv.add_global_callback('q', |s| s.quit());
    siv.run();
}
