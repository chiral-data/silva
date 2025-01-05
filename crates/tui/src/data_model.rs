use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;


pub struct Store {
    pub account_mgr: account::Manager,
    pub registry_mgr: registry::Manager,
    pub setting_mgr: settings::Manager,
    pub app_mgr: app::Manager,
    pub pod_type_mgr: pod_type::Manager,
    pub pod_mgr: pod::Manager,
    pub job_mgr: Arc<Mutex<job::Manager>>,
    pub project_sel: Option<project::Project>, 
}

impl std::default::Default for Store {
    fn default() -> Self {
        let app_mgr = app::Manager::new();
        let registry_mgr = registry::Manager::load().unwrap();
        let setting_mgr = settings::Manager::load().unwrap();
        let account_mgr = account::Manager::load().unwrap();
        let pod_type_mgr = pod_type::Manager::new();
        let pod_mgr = pod::Manager::new();
        let job_mgr = job::Manager::load().unwrap();

        Self { 
            account_mgr, registry_mgr, setting_mgr,
            app_mgr, pod_type_mgr, pod_mgr,
            project_sel: None,
            job_mgr: Arc::new(Mutex::new(job_mgr)), 
        }
    }
}

impl Store {
    pub fn update_project(&mut self, proj_dir: &Path) -> anyhow::Result<()> {
        let job_settings = job::Job::get_settings(proj_dir)?;
        let files = job_settings.files.all_files();
        let proj = project::Project { dir: proj_dir.to_path_buf(), files, jh_pre: None, jh_post: None };
        self.project_sel = Some(proj);

        Ok(())
    }
}



pub mod provider;
pub mod registry;
mod settings;
pub mod app;
pub mod account;
pub mod pod_type;
pub mod pod;
pub mod job;
mod project;
