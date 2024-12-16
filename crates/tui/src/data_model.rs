use std::{fs::File, io::Read, path::PathBuf};

use crate::constants;

pub struct Store {
    pub app_mgr: app::Manager,
    pub ac_mgr: account::Manager,
    pub pod_type_mgr: pod_type::Manager,
    pub pod_mgr: pod::Manager,
    pub proj_selected: Option<PathBuf>,
    pub jl_mgr: job_local::Manager,
}

fn get_file_content(fp: Option<PathBuf>) -> String {
    match fp {
        Some(path) => {
            let mut file_accounts = File::open(path).unwrap();
            let mut buf = String::new();
            let _read_size = file_accounts.read_to_string(&mut buf).unwrap();
            buf
        }
        None => {
            // let path = xdg_dirs.place_data_file(constants::FILE_ACCOUNTS).unwrap();
            // let _file_accounts = File::create(path).unwrap();
            String::from("")
        }
    }
}

impl std::default::Default for Store {
    fn default() -> Self {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(constants::APP_NAME).unwrap();
        let app_mgr = app::Manager::new();
        let ac_mgr = account::Manager::new(get_file_content(xdg_dirs.find_data_file(constants::FILE_ACCOUNTS)).as_str());
        let pod_type_mgr = pod_type::Manager::new();
        let pod_mgr = pod::Manager::new();
        let jl_mgr = job_local::Manager::new(get_file_content(xdg_dirs.find_data_file(constants::FILE_JOBS_LOCAL)).as_str());

        Self { app_mgr, ac_mgr, pod_type_mgr, pod_mgr, proj_selected: None, jl_mgr }
    }
}

mod common;

mod provider;
pub mod app;
mod account;
pub mod pod_type;
pub mod pod;
pub mod job;
pub mod job_local;
