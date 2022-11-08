extern crate app_dirs;
#[macro_use]
extern crate clap;
extern crate cursive;
extern crate indicatif;
extern crate num_cpus;
#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate toml;

mod config;
mod model;
mod report;
mod styles;
mod ui;
mod utils;
mod views;

use anyhow::Result;
use clap::{App, Arg};
use model::{MultiRepoHistory, Repo, RevWalkStrategy};
use std::env;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use utils::{find_project_file, find_repo_base_folder};

const MAX_NUMBER_OF_THREADS: usize = 18; //tests on a 36 core INTEL Xeon showed that parsing becomes slower again if more than 18 threads are used

fn main() -> Result<(), String> {
    let original_cwd = env::current_dir().expect("cwd not found");
    let matches = App::new("oper")
        .version(crate_version!())
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
            Arg::with_name("revwalk-strategy")
                .short("r")
                .long("revwalk")
                .value_name("strategy")
                .help("traverse the 1st parent only ('first' = fast) or all parents ('all' = slow)")
                .default_value("first")
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
        .arg(
            Arg::with_name("manifest")
                .short("x")
                .long("manifest")
                .help("include changes to the manifest repository")
        )
        .arg(
            Arg::with_name("report")
            .long("report")
            .value_name("file")
            .help("writes a report to a file given by <path> - supported formats: .csv, .ods, .xlsx")
            .takes_value(true)
        )
        .get_matches();

    let days = value_t!(matches.value_of("days"), u32).unwrap_or_else(|e| e.exit());
    let classifier = model::Classifier::new(
        days,
        matches.value_of("author"),
        matches.value_of("message"),
    );
    let cwd = Path::new(matches.value_of("cwd").unwrap());
    let revwalk_strategy = match matches.value_of("revwalk-strategy") {
        Some("first") => Ok(RevWalkStrategy::FirstParent),
        Some("all") => Ok(RevWalkStrategy::AllParents),
        _ => Err(format!("Unknown revwalk strategy given")),
    }?;

    do_main(
        &classifier,
        &revwalk_strategy,
        cwd,
        matches.is_present("manifest"),
        matches.value_of("report"),
    )
    .or_else(|e| Err(e.to_string()))
}

fn do_main(
    classifier: &model::Classifier,
    revwalk_strategy: &RevWalkStrategy,
    cwd: &Path,
    include_manifest: bool,
    report_file_path: Option<&str>,
) -> Result<()> {
    let config = config::read();

    env::set_current_dir(cwd)?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(std::cmp::min(num_cpus::get(), MAX_NUMBER_OF_THREADS))
        .build_global()
        .unwrap();

    let project_file = File::open(find_project_file()?)?;
    let repos = repos_from(&project_file, include_manifest)?;

    let history = MultiRepoHistory::from(repos, &classifier, revwalk_strategy)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    //TUI or report?
    match report_file_path {
        None => ui::show(history, config),
        Some(file) => {
            println!("Skipping UI - generating report...");
            report::generate(&history, file)?
        }
    }

    Ok(())
}

fn repos_from(
    project_file: &std::fs::File,
    include_manifest: bool,
) -> Result<Vec<Arc<Repo>>, io::Error> {
    let mut repos = Vec::new();

    let base_folder = find_repo_base_folder()?;
    for project in BufReader::new(project_file).lines() {
        let rel_path = project.expect("project.list read error");
        repos.push(Arc::new(Repo::from(base_folder.join(&rel_path), rel_path)));
    }

    if include_manifest {
        let rel_path = String::from(".repo/manifests");
        repos.push(Arc::new(Repo::from(base_folder.join(&rel_path), rel_path)));
    }

    Ok(repos)
}
