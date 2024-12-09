use std::path::PathBuf;


pub struct Store {
    pub account_mgr: account::Manager,
    pub setting_mgr: settings::Manager,
    pub app_mgr: app::Manager,
    pub pod_type_mgr: pod_type::Manager,
    pub pod_mgr: pod::Manager,
    pub proj_selected: Option<PathBuf>,
    pub job_mgr: job::Manager,
}

impl std::default::Default for Store {
    fn default() -> Self {
        let app_mgr = app::Manager::new();
        let setting_mgr = settings::Manager::load().unwrap();
        let account_mgr = account::Manager::load().unwrap();
        let pod_type_mgr = pod_type::Manager::new();
        let pod_mgr = pod::Manager::new();
        let job_mgr = job::Manager::load().unwrap();

        Self { 
            account_mgr, setting_mgr,
            app_mgr, pod_type_mgr, pod_mgr,
            proj_selected: None, 
            job_mgr, 
        }
    }
}

mod common;

mod provider;
mod settings;
pub mod app;
mod account;
pub mod pod_type;
pub mod pod;
pub mod job;
