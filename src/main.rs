#[macro_use]
extern crate clap;
extern crate cursive;

mod model;
mod table_view;
mod ui;
mod utils;

use clap::{App, Arg};
use model::{MultiRepoHistory, Repo};
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
                .default_value("10")
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

    do_main(days, cwd).or_else(|e| Err(String::from(e.description())))
}

fn do_main(days: usize, cwd: &Path) -> Result<(), io::Error> {
    env::set_current_dir(cwd).expect("changing cwd failed");
    let project_file = File::open(find_project_file()?)?;
    let history = build_history(project_file, days)?;

    //    println!("{:?}", history);
    ui::show(history);

    Ok(())
}

fn build_history(project_file: std::fs::File, days: usize) -> Result<MultiRepoHistory, io::Error> {
    let mut repos = Vec::new();

    let base_folder = find_repo_base_folder()?;
    for project in BufReader::new(project_file).lines() {
        let rel_path = project.expect("project.list read error");
        let abs_path = base_folder.join(&rel_path);
        repos.push(Rc::new(Repo::from(abs_path, rel_path)));
    }
    MultiRepoHistory::from_last_days(repos, days)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.description()))
}
