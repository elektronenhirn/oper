use crate::utils::{as_datetime, as_datetime_utc};
use chrono::{Datelike, Duration, Timelike};
use console::style;
use git2::{Commit, DiffFormat, Oid, Repository, Time};
use indicatif::{MultiProgress, ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

/// A history of commits across multiple repositories
pub struct MultiRepoHistory {
    pub repos: Vec<Arc<Repo>>,
    pub commits: Vec<RepoCommit>,
    pub max_width_repo: usize,
    pub max_width_committer: usize,
}

impl MultiRepoHistory {
    pub fn from(
        repos: Vec<Arc<Repo>>,
        classifier: &Classifier,
    ) -> Result<MultiRepoHistory, git2::Error> {
        let progress = MultiProgress::new();
        let progress_bars = (0..rayon::current_num_threads())
            .enumerate()
            .map(|(n, _)| {
                let pb = ProgressBar::hidden();
                pb.set_prefix(&n.to_string());
                pb.set_style(
                    ProgressStyle::default_spinner().template("[{prefix}] {wide_msg:.bold.dim}"),
                );
                progress.add(pb)
            })
            .collect::<Vec<ProgressBar>>();
        let overall_progress = ProgressBar::new(repos.len() as u64);
        overall_progress.set_style(
            ProgressStyle::default_bar()
                .template(" {spinner:.bold.cyan}  Scanned {pos} of {len} repositories"),
        );
        let overall_progress = progress.add(overall_progress);

        thread::spawn(move || {
            progress.join_and_clear().unwrap();
        });

        let mut commits: Vec<RepoCommit> = repos
            .par_iter()
            .progress_with(overall_progress)
            .filter_map(move |repo| {
                let progress_bar = &progress_bars[rayon::current_thread_index()?];
                progress_bar.set_message(&format!("Scanning {}", repo.rel_path));

                let progress_error = |msg: &str, error: &dyn std::error::Error| {
                    progress_bar.println(format!(
                        "{}: {}: {}",
                        style(&repo.rel_path).cyan(),
                        style(&msg).red(),
                        error
                    ));
                    progress_bar.inc(1);
                    progress_bar.set_message("Idle");
                };

                let git_repo = Repository::open(&repo.abs_path)
                    .map_err(|e| progress_error("Failed to open", &e))
                    .ok()?;

                let mut revwalk = git_repo
                    .revwalk()
                    .map_err(|e| progress_error("Failed create revwalk", &e))
                    .ok()?;
                revwalk.set_sorting(git2::Sort::TIME);

                revwalk
                    .push_head()
                    .map_err(|e| progress_error("Failed query history", &e))
                    .ok()?;

                let mut commits = Vec::new();
                for commit_id in revwalk {
                    let commit = commit_id
                        .and_then(|commit_id| git_repo.find_commit(commit_id))
                        .map_err(|e| progress_error("Failed find commit", &e))
                        .ok()?;
                    let (include, abort) = classifier.classify(&commit);
                    if include {
                        commits.push(RepoCommit::from(repo.clone(), &commit));
                    }
                    if abort {
                        break;
                    }
                }
                progress_bar.set_message("Idle");
                Some(commits)
            })
            .flatten()
            .collect();

        let max_width_repo = repos.iter().map(|r| r.description.len()).max().unwrap_or(0);
        let max_width_committer = commits.iter().map(|c| c.committer.len()).max().unwrap_or(0);

        commits.sort_unstable_by(|a, b| a.timestamp.cmp(&b.timestamp).reverse());
        Ok(MultiRepoHistory {
            repos,
            commits,
            max_width_repo,
            max_width_committer,
        })
    }
}

impl fmt::Debug for MultiRepoHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        println!("Commits: {}", self.commits.len());
        for commit in &self.commits {
            write!(f, "{:?}", commit)?;
        }
        Ok(())
    }
}

/// representation of a local git repository
pub struct Repo {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub description: String,
}

impl Repo {
    pub fn from(abs_path: PathBuf, rel_path: String) -> Repo {
        let description = abs_path.file_name().unwrap().to_str().unwrap().into();
        Repo {
            abs_path,
            rel_path,
            description,
        }
    }
}

/// representation of a git commit associated
/// with a local git repository
#[derive(Clone)]
pub struct RepoCommit {
    pub repo: Arc<Repo>,
    pub timestamp: Time,
    pub summary: String,
    pub author: String,
    pub committer: String,
    pub commit_id: Oid,
    pub message: String,
}

impl RepoCommit {
    pub fn from(repo: Arc<Repo>, commit: &Commit) -> RepoCommit {
        let timestamp = commit.time();
        let summary = commit.summary().unwrap_or("None");
        let author = commit.author().name().unwrap_or("None").into();
        let committer = commit.committer().name().unwrap_or("None").into();
        let commit_id = commit.id();
        let message = commit.message().unwrap_or("").to_string();

        RepoCommit {
            repo,
            timestamp,
            summary: summary.into(),
            author,
            committer,
            commit_id,
            message,
        }
    }

    pub fn time_as_str(&self) -> String {
        let date_time = as_datetime(&self.timestamp);
        let offset = Duration::seconds(i64::from(date_time.offset().local_minus_utc()));

        format!(
            "{:04}-{:02}-{:02} {:02}:{:02} {:+02}{:02}",
            date_time.year(),
            date_time.month(),
            date_time.day(),
            date_time.hour(),
            date_time.minute(),
            offset.num_hours(),
            offset.num_minutes() - offset.num_hours() * 60
        )
    }

    pub fn diff(&self) -> Result<String, git2::Error> {
        let git_repo = Repository::open(&self.repo.abs_path)?;
        let commit = git_repo.find_commit(self.commit_id)?;
        let a = if commit.parents().len() == 1 {
            let parent = commit.parent(0)?;
            Some(parent.tree()?)
        } else {
            None
        };
        let b = commit.tree()?;
        let diff = git_repo.diff_tree_to_tree(a.as_ref(), Some(&b), None)?;
        let mut as_text = String::default();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            match line.origin() {
                ' ' | '+' | '-' => as_text += &line.origin().to_string(),
                _ => {}
            }
            as_text += std::str::from_utf8(line.content()).unwrap();
            true
        })?;
        Ok(as_text)
    }
}

impl fmt::Debug for RepoCommit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {:10.10} {:10.10} {}",
            self.time_as_str(),
            self.repo.description,
            self.committer,
            self.summary
        )
    }
}

pub struct Classifier<'a> {
    age: u32,
    author: Option<&'a str>,
    message: Option<String>,
}

impl<'a> Classifier<'a> {
    pub fn new(age: u32, author: Option<&'a str>, message: Option<&'a str>) -> Classifier<'a> {
        Classifier {
            age,
            author,
            message: message.map(str::to_lowercase),
        }
    }
}

impl<'a> Classifier<'a> {
    fn classify(&self, commit: &Commit) -> (bool, bool) {
        let utc = as_datetime_utc(&commit.time());
        let diff = chrono::Utc::now().signed_duration_since(utc);
        let include = diff.num_days() as u32 <= self.age;
        let (mut include, abort) = (include, !include);

        if let Some(ref message) = self.message {
            let cm = commit.message().unwrap_or("").to_ascii_lowercase();
            include &= cm.contains(message);
        }

        if let Some(ref author) = self.author {
            let ca = commit.author().name().unwrap_or("").to_ascii_lowercase();
            include &= ca.contains(author);
        }

        (include, abort)
    }
}
