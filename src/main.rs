use std::io;
use std::path::Path;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[macro_use]
extern crate clap;
use clap::{App, Arg};

mod utils;
use utils::{find_project_file, find_repo_base_folder};

fn main() -> Result<(), String> {
    let mut original_cwd = env::current_dir().expect("cwd not found");
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

    let days =
        value_t!(matches.value_of("days"), usize).unwrap_or_else(|e| e.exit());
    let cwd = Path::new(matches.value_of("cwd").unwrap());

    do_main(days, cwd).or_else(|e| Err( String::from(e.description())))
}

fn do_main(days: usize, cwd: &Path) -> Result<(), io::Error> {
    env::set_current_dir(cwd).expect("changing cwd failed");
    let project_file = find_project_file()?;
    let base_folder = find_repo_base_folder()?;
    let file = File::open(project_file)?;
    for line in BufReader::new(file).lines() {
            let git = base_folder.join(line.expect("project.list read error"));
            println!("{:?}", git);
    }
    Ok(())
}