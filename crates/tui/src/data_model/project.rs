use std::path::{Path, PathBuf};

pub struct Project {
    dir: PathBuf,
    job_settings: super::job::settings::Settings
}

impl Project {
    pub fn new(dir: PathBuf, job_settings: super::job::settings::Settings) -> Self {
        Self { dir, job_settings }
    }

    pub fn get_dir(&self) -> &Path {
        self.dir.as_path()
    }

    pub fn get_job_settings(&self) -> &super::job::settings::Settings {
        &self.job_settings
    }

    pub fn get_files(&self) -> Vec<String> {
        self.job_settings.files.all_files()
    }

    pub fn get_project_name(&self) -> anyhow::Result<String> {
        let proj_name = self.dir.file_name()
            .ok_or(anyhow::Error::msg("no file name for project "))?
            .to_str()
            .ok_or(anyhow::Error::msg("osString to str error"))?;
        let proj_parent =self.dir.parent()
            .map(|p| p.file_name().unwrap().to_str().unwrap_or(""))
            .unwrap_or("");

        Ok(format!("{proj_parent}_{proj_name}"))
    }

    pub fn get_docker_image_name(&self) -> anyhow::Result<String> {
        let proj_name = self.get_project_name()?;
        Ok(format!("{proj_name}:latest").to_lowercase())
    }
}

#[derive(Default)]
pub struct Manager {
    /// jh for pre-processing
    jh_pre: Option<tokio::task::JoinHandle<()>>,
    /// jh for post-processing
    jh_post: Option<tokio::task::JoinHandle<()>>,
}

impl Manager {
    pub fn is_pre_processing_finished(&mut self) -> bool {
        if let Some(jh) = self.jh_pre.as_ref() {
            if jh.is_finished() {
                self.jh_pre = None;
                true
            } else { false }
        } else { true }
    }

    pub fn is_post_processing_finished(&mut self) -> bool {
        if let Some(jh) = self.jh_post.as_ref() {
            if jh.is_finished() {
                self.jh_post = None;
                true
            } else { false }
        } else { true }
    }

    pub fn add_pre_processing(&mut self, jh: tokio::task::JoinHandle<()>) {
        self.jh_pre = Some(jh);
    }

    pub fn add_post_processing(&mut self, jh: tokio::task::JoinHandle<()>) {
        self.jh_post = Some(jh);
    }
}
