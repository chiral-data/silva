use std::path::{Path, PathBuf};

use crate::constants;

pub fn get_child_dirs<P: AsRef<Path>>(dir: P) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(&dir)
        .map_err(|e| format!("read dir {:?} error {e}", dir.as_ref())).unwrap()
        .filter_map(|entry| match entry {
            Ok(e) => {
                if e.path().is_dir() {
                    e.path().to_str().map(PathBuf::from)
                } else { None }
            }
            Err(_) => None
        })
}

#[inline]
fn silva_project_dir() -> directories::ProjectDirs {
    directories::ProjectDirs::from("com", constants::ORG_NAME,  constants::APP_NAME)
        .ok_or(anyhow::Error::msg("error get silva project dir"))
        .unwrap()
}

pub fn data_dir() -> PathBuf {
    let home_dir = silva_project_dir();
    home_dir.data_dir().to_path_buf()
}

// When the project dir contains a README.md file
fn add_project_dir(dir: &Path, project_dirs: &mut Vec<PathBuf>) {
    let readme_file = dir.join("README.md");
    if readme_file.exists() {
        project_dirs.push(dir.to_path_buf());
    } else {
        for new_dir in get_child_dirs(dir) {
            add_project_dir(&new_dir, project_dirs);
        }
    }
}

pub fn get_project_dirs() -> Vec<PathBuf> {
    let projects_home_string = std::env::var_os(constants::SILVA_PROJECTS_HOME).unwrap();
    let mut project_dirs = Vec::<PathBuf>::new();
    for new_dir in get_child_dirs(PathBuf::from(&projects_home_string)) {
        add_project_dir(&new_dir, &mut project_dirs);
    }

    project_dirs
}

pub fn get_tutorial_dirs() -> Vec<PathBuf> {
    let data_dir = data_dir();
    let mut tutorial_dirs = Vec::<PathBuf>::new();
    for new_dir in get_child_dirs(data_dir.join(format!("v{}", constants::TAG)).join(format!("application-examples-{}", constants::TAG))) {
        add_project_dir(&new_dir, &mut tutorial_dirs);
    }

    tutorial_dirs
}
