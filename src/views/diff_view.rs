use crate::model::RepoCommit;
use crate::styles::{BLUE, GREEN, LIGHT_BLUE, MAGENTA, RED, WHITE, YELLOW};
use crate::views::ListView;
use cursive::theme::ColorStyle;
use cursive::view::ViewWrapper;

pub struct DiffView {
    list_view: ListView,
}

impl DiffView {
    pub fn empty() -> Self {
        DiffView {
            list_view: ListView::new(),
        }
    }

    #[rustfmt::skip]
    pub fn set_commit(self: &mut Self, entry: &RepoCommit) {
        self.list_view = ListView::new();

        self.list_view.insert_colorful_string(format!("Repo:       {}", entry.repo.rel_path), *RED);
        self.list_view.insert_colorful_string(format!("Id:         {}", entry.commit_id), *BLUE);
        self.list_view.insert_colorful_string(format!("Author:     {}", entry.author), *LIGHT_BLUE);
        self.list_view.insert_colorful_string(format!("Commit:     {}", entry.committer), *GREEN);
        self.list_view.insert_colorful_string(format!("CommitDate: {}\n", entry.time_as_str()), *BLUE);
        self.list_view.insert_colorful_string(entry.message.clone(), *WHITE);
        self.list_view.insert_string("---".to_string());

        let diff = entry.diff();

        for (sigil, line) in &diff {
            let mut combined = match sigil {
                ' ' | '+' | '-' => sigil.to_string() + line,
                'F' => "\n".to_string() + line,
                _ => line.to_string(),
            };
            Self::trim_newline(&mut combined);
            self.list_view.insert_colorful_string(combined, Self::style_of(*sigil));
        }
    }

    fn trim_newline(s: &mut String) {
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
    }

    fn style_of(sigil: char) -> ColorStyle {
        match sigil {
            ' ' => *BLUE,
            '+' => *GREEN,
            '-' => *RED,
            'F' => *YELLOW,
            'H' => *MAGENTA,
            _ => *BLUE,
        }
    }
}

impl ViewWrapper for DiffView {
    type V = ListView;

    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        Some(f(&self.list_view))
    }

    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        Some(f(&mut self.list_view))
    }
}
