use crate::model::RepoCommit;
use cursive::theme::{ColorStyle, Style};
use cursive::traits::Finder;
use cursive::traits::Identifiable;
use cursive::utils::span::SpannedString;
use cursive::view::ViewWrapper;
use cursive::views::{IdView, ScrollView, TextView, ViewRef};

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
    pub fn set_commit(self: &mut Self, entry: &RepoCommit, show_diff: bool) {
        let mut text = SpannedString::<Style>::plain("");

        text.append_styled(format!("Repo:       {}\n", entry.repo.rel_path), ColorStyle::primary() );
        text.append_styled(format!("Id:         {}\n", entry.commit_id),     ColorStyle::primary());
        text.append_styled(format!("Author:     {}\n", entry.author),        ColorStyle::tertiary() );
        text.append_styled(format!("Commit:     {}\n", entry.committer),     ColorStyle::tertiary() );
        text.append_styled(format!("CommitDate: {}\n", entry.time_as_str()), ColorStyle::secondary() );
        text.append("\n");

        text.append(&entry.message);
        text.append("---\n");

        if show_diff {
            for (sigil, line) in &entry.diff() {
                let combined = match sigil {
                    ' ' | '+' | '-' => sigil.to_string() + line,
                    _ => line.to_string(),
                };
                text.append_styled(combined, Self::style_of(*sigil), );
            }
        } else {
            text.append_styled("\nPress <ENTER> to load diff", ColorStyle::tertiary());
        }

        let mut text_view: ViewRef<TextView> = self.scroll_view.find_id("diff").unwrap();
        text_view.set_content(text);
    }

    fn style_of(sigil: char) -> ColorStyle {
        match sigil {
            ' ' => ColorStyle::primary(),
            '+' => ColorStyle::tertiary(),
            '-' => ColorStyle::secondary(),
            _ => ColorStyle::primary(),
        }
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
