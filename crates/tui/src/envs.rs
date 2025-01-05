//! Environmental Variables
//!

use std::{env, path::PathBuf};



pub const SILVA_PROJECTS_HOME: &str = "SILVA_PROJECTS_HOME";
pub fn get_projects_home() -> Vec<PathBuf> {
    let projects_home_string = env::var_os(SILVA_PROJECTS_HOME).unwrap();
    projects_home_string.into_string().unwrap()
        .split(';')
        .map(PathBuf::from)
        .collect()
}


