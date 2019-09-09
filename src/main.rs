#[macro_use]
extern crate clap;
extern crate cursive;
extern crate indicatif;

mod model;
mod table_view;
mod ui;
mod utils;

use clap::{App, Arg};
use indicatif::ProgressBar;
use model::{
    AgeClassifier, AuthorClassifier, CommitClassifier, MessageClassifier, MultiRepoHistory, Repo,
};
use std::convert::Into;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::rc::Rc;
use utils::{find_project_file, find_repo_base_folder};

fn main() -> Result<(), String> {
    let original_cwd = env::current_dir().expect("cwd not found");
    let matches = App::new("oper")
        .version("0.1.0")
        .author("Florian Bramer <elektronenhirn@gmail.com>")
        .about("git-repo history tool")
        .arg(
            Arg::with_name("days")
                .short("d")
                .long("days")
                .value_name("days")
                .help("include history of the last <n> days")
                .default_value("100")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("author")
                .short("a")
                .long("author")
                .value_name("pattern")
                .help(
                    "only include commits where author's name contains <pattern> (case insensitive)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("message")
                .short("m")
                .long("message")
                .value_name("pattern")
                .help("only include commits where message contains <pattern> (case insensitive)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cwd")
                .short("C")
                .long("cwd")
                .value_name("cwd")
                .help("change working directory (mostly useful for testing)")
                .default_value(original_cwd.to_str().unwrap())
                .takes_value(true),
        )
        .get_matches();

    let days = value_t!(matches.value_of("days"), usize).unwrap_or_else(|e| e.exit());
    let cwd = Path::new(matches.value_of("cwd").unwrap());

    let mut classifiers = Vec::<&dyn CommitClassifier>::new();

    let age_classifier = AgeClassifier(days);
    classifiers.push(&age_classifier);

    let author_classifier = matches
        .value_of("author")
        .map(Into::into)
        .map(AuthorClassifier);
    if let Some(ref c) = author_classifier {
        classifiers.push(c);
    }

    let message_classifier = matches
        .value_of("message")
        .map(Into::into)
        .map(MessageClassifier);
    if let Some(ref c) = message_classifier {
        classifiers.push(c);
    }

    do_main(classifiers, cwd).or_else(|e| Err(e.description().into()))
}

fn do_main(classifiers: Vec<&dyn CommitClassifier>, cwd: &Path) -> Result<(), io::Error> {
    env::set_current_dir(cwd).expect("changing cwd failed");
    let project_file = File::open(find_project_file()?)?;
    println!("Collecting histories from repo repositories...");
    let repos = repos_from(&project_file)?;
    let progress_bar = ProgressBar::new(repos.len() as u64);

    let history = MultiRepoHistory::from(repos, classifiers, &progress_bar)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.description()))?;

    //    println!("{:?}", history);
    ui::show(history);

    Ok(())
}

fn repos_from(project_file: &std::fs::File) -> Result<Vec<Rc<Repo>>, io::Error> {
    let mut repos = Vec::new();

    let base_folder = find_repo_base_folder()?;
    for project in BufReader::new(project_file).lines() {
        let rel_path = project.expect("project.list read error");
        let abs_path = base_folder.join(&rel_path);
        repos.push(Rc::new(Repo::from(abs_path, rel_path)));
    }

    Ok(repos)
}
