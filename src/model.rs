use crate::utils::{as_datetime, as_datetime_utc};
use chrono::{Datelike, Duration, Timelike};
use git2::{Commit, Oid, Repository, Time};
use indicatif::ProgressBar;
use std::cmp;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

/// A history of commits across multiple
/// repositories
pub struct MultiRepoHistory {
    pub repos: Vec<Rc<Repo>>,
    pub commits: Vec<RepoCommit>,
    pub max_width_repo: usize,
    pub max_width_committer: usize,
}

pub struct CommitClassification {
    pub include_commit: bool,
    pub abort_walk: bool,
}

impl MultiRepoHistory {
    pub fn from_last_days(
        repos: Vec<Rc<Repo>>,
        days: usize,
        progress: &ProgressBar,
    ) -> Result<MultiRepoHistory, git2::Error> {
        Self::from(
            repos,
            &|commit| {
                let utc = as_datetime_utc(&commit.time());
                let diff = chrono::Utc::now().signed_duration_since(utc);
                let include_commit = diff.num_days() <= days as i64;
                CommitClassification {
                    include_commit,
                    abort_walk: !include_commit,
                }
            },
            &progress,
        )
    }

    pub fn from(
        repos: Vec<Rc<Repo>>,
        classify: &Fn(&Commit) -> CommitClassification,
        progress: &ProgressBar,
    ) -> Result<MultiRepoHistory, git2::Error> {
        let mut commits = Vec::new();
        let mut max_width_repo = 0;
        let mut max_width_committer = 0;

        for repo in &repos {
            max_width_repo = cmp::max(max_width_repo, repo.description.len());
            let git_repo = Repository::open(&repo.abs_path)?;
            let mut revwalk = git_repo.revwalk()?;
            revwalk.set_sorting(git2::Sort::TIME);
            revwalk.push_head()?;
            for commit_id in revwalk {
                let commit_id = commit_id?;
                let commit = git_repo.find_commit(commit_id)?;

                let classification = classify(&commit);
                if classification.include_commit {
                    let entry = RepoCommit::from(repo.clone(), &commit);
                    max_width_committer = cmp::max(max_width_committer, entry.committer.len());
                    commits.push(entry);
                }
                if classification.abort_walk {
                    break;
                }
            }
            progress.inc(1);
        }
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
    pub repo: Rc<Repo>,
    pub timestamp: Time,
    pub summary: String,
    pub author: String,
    pub committer: String,
    pub commit_id: Oid,
    pub message: String,
}

impl RepoCommit {
    pub fn from(repo: Rc<Repo>, commit: &Commit) -> RepoCommit {
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
