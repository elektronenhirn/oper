use crate::utils::{as_datetime, as_datetime_utc};
use chrono::{Datelike, Duration, Timelike};
use git2::{Commit, Repository, Time};
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::cmp;

/// A history of commits across multiple
/// repositories
pub struct MultiRepoHistory {
    repos: Vec<Rc<Repo>>,
    pub commits: Vec<Entry>,
    pub max_width_repo: usize,
    pub max_width_committer: usize,
}

impl MultiRepoHistory {
    pub fn from_last_days(
        repos: Vec<Rc<Repo>>,
        days: usize,
    ) -> Result<MultiRepoHistory, git2::Error> {
        Self::from(repos, &|commit| {
            let utc = as_datetime_utc(&commit.time());
            let diff = chrono::Utc::now().signed_duration_since(utc);
            diff.num_days() <= days as i64
        })
    }

    pub fn from(
        repos: Vec<Rc<Repo>>,
        pred: &Fn(&Commit) -> bool,
    ) -> Result<MultiRepoHistory, git2::Error> {
        let mut commits = Vec::new();
        let mut max_width_repo = 0;
        let mut max_width_committer = 0;

        for repo in &repos {
            max_width_repo = cmp::max(max_width_repo, repo.description.len());
            let git_repo = Repository::open(&repo.path)?;
            let mut revwalk = git_repo.revwalk()?;
            revwalk.set_sorting(git2::Sort::TIME);
            revwalk.push_head()?;
            for commit_id in revwalk {
                let commit_id = commit_id?;
                let commit = git_repo.find_commit(commit_id)?;
                if !pred(&commit) {
                    continue;
                }
                let entry = Entry::from(repo.clone(), &commit);
                max_width_committer = cmp::max(max_width_committer, entry.committer.len());
                commits.push(entry);
            }
        }
        commits.sort_unstable_by(|a,b|{
          a.timestamp.cmp(&b.timestamp).reverse()
        });
        Ok(MultiRepoHistory { repos, commits, max_width_repo, max_width_committer })
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
    path: PathBuf,
    pub description: String,
}

impl Repo {
    pub fn from(path: PathBuf) -> Repo {
        let description = String::from(path.file_name().unwrap().to_str().unwrap());
        Repo { path, description }
    }
}

/// representation of a git commit associated
/// with a local git repository
#[derive(Clone)]
pub struct Entry {
    pub repo: Rc<Repo>,
    pub timestamp: Time,
    pub summary: String,
    pub author: String,
    pub committer: String,
}

impl Entry {
    pub fn from(repo: Rc<Repo>, commit: &Commit) -> Entry {
        let timestamp = commit.time();
        let summary = commit.summary().unwrap_or("None");
        let author = String::from(commit.author().name().unwrap_or("None"));
        let committer = String::from(commit.committer().name().unwrap_or("None"));
        Entry {
            repo,
            timestamp,
            summary: String::from(summary),
            author,
            committer,
        }
    }

    pub fn time_as_str(&self) -> String {
        let date_time = as_datetime(&self.timestamp);
        let offset = Duration::seconds(date_time.offset().local_minus_utc() as i64);

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

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {:10.10} {:10.10} {}\n",
            self.time_as_str(),
            self.repo.description,
            self.committer,
            self.summary
        )
    }
}