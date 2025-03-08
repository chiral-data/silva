use std::path::PathBuf;

use crate::constants;

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
