//! Environmental Variables
//!

use std::{env, path::PathBuf};

const SILVA_PROJECTS_HOME: &str = "SILVA_PROJECTS_HOME";

pub fn setup() {
    // let home_dir = home::home_dir().unwrap();
    //env::set_var(SILVA_PROJECTS_HOME, home_dir.join("Downloads").join("sylvest_projects"));
    env::set_var(SILVA_PROJECTS_HOME, PathBuf::from(".").join("examples"))
}

pub fn get_projects_home() -> PathBuf {
    PathBuf::from(env::var_os(SILVA_PROJECTS_HOME).unwrap())
}

