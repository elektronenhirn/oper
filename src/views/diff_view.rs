use crate::model::RepoCommit;
use crate::styles::{BLUE, GREEN, LIGHT_BLUE, MAGENTA, RED, WHITE, YELLOW};
use crate::views::ListView;
use cursive::theme::ColorStyle;
use cursive::view::ViewWrapper;
use std::process::Command;

pub struct DiffView {
    list_view: ListView,
    commit: Option<RepoCommit>,
}

impl DiffView {
    pub fn empty() -> Self {
        DiffView {
            list_view: ListView::new(),
            commit: None,
        }
    }

    pub fn set_commit(self: &mut Self, entry: &RepoCommit) {
        self.commit = Some(entry.clone());

        self.list_view = ListView::new();
        self.list_view
            .insert_colorful_string(format!("Repo:       {}", entry.repo.rel_path), *RED);

        // we first add the output of git show without diff (does not work nicely for merge
        // commits yet - but support will come in never versions of git-show...)
        self.add_git_show_output(&entry);

        self.list_view
            .insert_colorful_string("―――".to_string(), *YELLOW);

        // now at the diff output between the given commit and its first parent
        // this will then also work nicely with merge commits
        self.add_git_diff_output(&entry);
    }

    #[rustfmt::skip]
    fn add_git_show_output(self: &mut Self, entry: &RepoCommit){
        let output = Command::new("git")
                     .current_dir(&entry.repo.abs_path)
                     .arg("--no-pager")
                     .arg("show")
                     .arg("--patch-with-stat")
                     .arg("--encoding=UTF-8")
                     .arg("--pretty=fuller")
                     .arg("--no-color")
                     .arg("--no-patch")
                     .arg(format!("{}", entry.commit_id))
                     .output()
                     .expect("Failed to execute git-show command. git not installed?");

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            self.list_view.insert_colorful_string(line.to_string(), Self::color_of(line));
        }
    }

    #[rustfmt::skip]
    fn add_git_diff_output(self: &mut Self, entry: &RepoCommit){
        let output = Command::new("git")
                     .current_dir(&entry.repo.abs_path)
                     .arg("--no-pager")
                     .arg("diff")
                     .arg("--patch-with-stat")
                     .arg("--encoding=UTF-8")
                     .arg("--pretty=fuller")
                     .arg("--patch-with-stat")
                     .arg("--no-color")
                     .arg(format!("{}..{}^", entry.commit_id, entry.commit_id))
                     .output()
                     .expect("Failed to execute git-show command. git not installed?");

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            self.list_view.insert_colorful_string(line.to_string(), Self::color_of(line));
        }
    }

    fn color_of(line: &str) -> ColorStyle {
        let color_coding = [
            ("commit ", *BLUE),
            ("Author: ", *LIGHT_BLUE),
            ("AuthorDate: ", *YELLOW),
            ("Commit: ", *MAGENTA),
            ("CommitDate: ", *YELLOW),
            ("---", *YELLOW),
            ("+++", *YELLOW),
            ("new ", *YELLOW),
            ("rename", *YELLOW),
            ("diff", *YELLOW),
            ("@", *MAGENTA),
            ("+", *GREEN),
            ("-", *RED),
        ];

        for cc in &color_coding {
            if line.starts_with(cc.0) {
                return cc.1;
            }
        }
        return *WHITE;
    }

    pub fn commit(self: &Self) -> &Option<RepoCommit> {
        &self.commit
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
