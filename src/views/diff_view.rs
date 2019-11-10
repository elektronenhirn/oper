use crate::model::RepoCommit;
use cursive::theme::{ColorStyle, Style};
use cursive::traits::Finder;
use cursive::traits::Identifiable;
use cursive::utils::span::SpannedString;
use cursive::view::ViewWrapper;
use cursive::views::{ScrollView, IdView, TextView, ViewRef};

pub struct DiffView {
    scroll_view: ScrollView<IdView<TextView>>,
}

impl DiffView {
    pub fn empty() -> Self {
        DiffView {
            scroll_view: ScrollView::new(TextView::empty().with_id("diff")),
        }
    }

    #[rustfmt::skip]
  pub fn set_commit(self: &mut Self, entry: &RepoCommit){
    let mut text = SpannedString::<Style>::plain("");

    text.append_styled(format!("Repo:       {}\n", entry.repo.rel_path), ColorStyle::primary() );
    text.append_styled(format!("Id:         {}\n", entry.commit_id),     ColorStyle::primary());
    text.append_styled(format!("Author:     {}\n", entry.author),        ColorStyle::tertiary() );
    text.append_styled(format!("Commit:     {}\n", entry.committer),     ColorStyle::tertiary() );
    text.append_styled(format!("CommitDate: {}\n", entry.time_as_str()), ColorStyle::secondary() );
    text.append("\n");

    text.append(&entry.message);
    text.append("---\n");
    text.append(&entry.diff().unwrap_or_else(|_| "<no diff>".to_string()));

    let mut text_view: ViewRef<TextView> = self.scroll_view.find_id("diff").unwrap();
    text_view.set_content(text);
  }
}

impl ViewWrapper for DiffView {
    type V = ScrollView<IdView<TextView>>;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        Some(f(&self.scroll_view))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        Some(f(&mut self.scroll_view))
    }
}
