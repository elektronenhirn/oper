use crate::utils::{as_datetime, as_datetime_utc};
use chrono::{Datelike, Duration, Timelike};
use git2::{Commit, DiffFormat, Oid, Repository, Time};
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

impl MultiRepoHistory {
    pub fn from(
        repos: Vec<Rc<Repo>>,
        classifiers: Vec<&dyn CommitClassifier>,
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
                let classification =
                    classifiers
                        .iter()
                        .fold(CommitClassification::default(), |sum, x| {
                            let classification = x.classify(&commit);
                            CommitClassification {
                                abort_walk: sum.abort_walk && classification.abort_walk,
                                include: sum.include && classification.include,
                            }
                        });
                //                let classification = classify(&commit);
                if classification.include {
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

pub struct CommitClassification {
    pub include: bool,
    pub abort_walk: bool,
}

impl CommitClassification {
    pub fn default() -> Self {
        Self {
            include: true,
            abort_walk: false,
        }
    }
}

pub trait CommitClassifier {
    fn classify(&self, commit: &Commit) -> CommitClassification;
}

pub struct AgeClassifier(pub usize);

impl CommitClassifier for AgeClassifier {
    fn classify(&self, commit: &Commit) -> CommitClassification {
        let utc = as_datetime_utc(&commit.time());
        let diff = chrono::Utc::now().signed_duration_since(utc);
        let include = diff.num_days() <= self.0 as i64;
        CommitClassification {
            include,
            abort_walk: !include,
        }
    }
}

pub struct AuthorClassifier(pub String);

impl CommitClassifier for AuthorClassifier {
    fn classify(&self, commit: &Commit) -> CommitClassification {
        let author = commit.author().name().unwrap_or("").to_ascii_lowercase();
        CommitClassification {
            include: author.contains(&self.0.to_lowercase()),
            abort_walk: false,
        }
    }
}

pub struct MessageClassifier(pub String);

impl CommitClassifier for MessageClassifier {
    fn classify(&self, commit: &Commit) -> CommitClassification {
        let author = commit.message().unwrap_or("").to_ascii_lowercase();
        CommitClassification {
            include: author.contains(&self.0.to_lowercase()),
            abort_walk: false,
        }
    }
}
