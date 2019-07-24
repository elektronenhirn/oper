use std::io;
use std::fs::{self};
use std::path::Path;
use std::env;
use std::path::PathBuf;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[macro_use]
extern crate clap;

use clap::{App, Arg};

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
    let file = File::open(project_file)?;
    for line in BufReader::new(file).lines() {
            println!("{}", line.expect("project.list read error"));
    }
    Ok(())
}

fn find_project_file() -> Result<PathBuf, io::Error> {
    let project_file = find_repo_folder()?.join("project.list");
    if project_file.is_file() {
        Ok(project_file)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "no project.list in .repo found"))
    }
}

fn find_repo_folder() -> Result<PathBuf, io::Error>{
    let cwd = env::current_dir()?;
    for parent in cwd.ancestors() {
        for entry in fs::read_dir(&parent)? {
            let entry = entry?;
            if entry.file_name() == ".repo" {
                return Ok(entry.path())
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::Other, "no .repo folder found"))
}