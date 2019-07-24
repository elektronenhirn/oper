use std::io;
use std::fs::{self};
use std::path::PathBuf;
use std::env;

pub fn find_project_file() -> Result<PathBuf, io::Error> {
    let project_file = find_repo_folder()?.join("project.list");
    if project_file.is_file() {
        Ok(project_file)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "no project.list in .repo found"))
    }
}

pub fn find_repo_folder() -> Result<PathBuf, io::Error>{
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