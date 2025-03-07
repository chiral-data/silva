use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::utils;


pub struct Store {
    pub project_dir: directories::ProjectDirs,
    pub account_mgr: account::Manager,
    pub registry_mgr: registry::Manager,
    pub setting_mgr: settings::Manager,
    pub app_mgr: app::Manager,
    pub pod_type_mgr: pod_type::Manager,
    pub pod_mgr: pod::Manager,
    pub job_mgr: Arc<Mutex<job::Manager>>,
    pub project_sel: Option<(project::Project, project::Manager)>, 
}

impl std::default::Default for Store {
    fn default() -> Self {
        let project_dir = utils::file::silva_project_dir();
        let app_mgr = app::Manager::new();
        let registry_mgr = registry::Manager::load().unwrap();
        let setting_mgr = settings::Manager::load().unwrap();
        let account_mgr = account::Manager::load().unwrap();
        let pod_type_mgr = pod_type::Manager::new();
        let pod_mgr = pod::Manager::new();
        let job_mgr = job::Manager::load().unwrap();


        let data_dir = project_dir.data_dir(); 
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir).unwrap();
        }

        Self { 
            project_dir,
            account_mgr, registry_mgr, setting_mgr,
            app_mgr, pod_type_mgr, pod_mgr,
            project_sel: None,
            job_mgr: Arc::new(Mutex::new(job_mgr)), 
        }
    }
}

impl Store {
    pub fn update_project(&mut self, proj_dir: &Path) -> anyhow::Result<()> {
        self.pod_type_mgr.pod_type_id_selected = None;
        self.pod_mgr.pod_id_selected = None;

        let job_settings = job::Job::get_settings(proj_dir)?;
        if let Some(dok) = job_settings.dok.as_ref() {
            if let Some(plan) = dok.plan.as_ref() {
                self.pod_type_mgr.pod_type_id_selected = Some(pod_type::ids::DOK);
                match plan {
                    sacloud_rs::api::dok::params::Plan::V100 => self.pod_mgr.pod_id_selected = Some(pod::ids::DOK_V100),
                    sacloud_rs::api::dok::params::Plan::H100GB80 => self.pod_mgr.pod_id_selected = Some(pod::ids::DOK_H100),
                    sacloud_rs::api::dok::params::Plan::H100GB20 => todo!()
                }
            }
        }

        let proj = project::Project::new(proj_dir.to_path_buf(), job_settings);
        let proj_mgr = project::Manager::default();
        self.project_sel = Some((proj, proj_mgr));

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
pub mod project;
