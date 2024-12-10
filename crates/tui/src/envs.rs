//! Environmental Variables
//!

use std::{env, path::PathBuf};



pub const SILVA_PROJECTS_HOME: &str = "SILVA_PROJECTS_HOME";
pub fn get_projects_home() -> PathBuf {
    PathBuf::from(env::var_os(SILVA_PROJECTS_HOME).unwrap())
}


