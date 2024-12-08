//! Environmental Variables
//!

use std::{env, path::PathBuf};

const SILVA_PROJECTS_HOME: &str = "SILVA_PROJECTS_HOME";

const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS";
const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME";
const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD";

pub fn setup() {
    env::set_var(SILVA_PROJECTS_HOME, PathBuf::from(".").join("examples"))
}

pub fn get_projects_home() -> PathBuf {
    PathBuf::from(env::var_os(SILVA_PROJECTS_HOME).unwrap())
}

/// get parameters of the container registry of Sakura Internet
/// which is necessary for using Sakura Internet DOK service
pub fn get_sakura_container_registry() -> (String, String, String) {
    (
        env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS).unwrap()
            .to_str().unwrap()
            .to_string(),
        env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME).unwrap()
            .to_str().unwrap()
            .to_string(),
        env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD).unwrap()
            .to_str().unwrap()
            .to_string(),
    )
}


