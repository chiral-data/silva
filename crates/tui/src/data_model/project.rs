use std::path::{Path, PathBuf};

pub struct Project {
    dir: PathBuf,
    files: Vec<String>,
    /// jh for pre-processing
    jh_pre: Option<tokio::task::JoinHandle<()>>,
    /// jh for post-processing
    jh_post: Option<tokio::task::JoinHandle<()>>,
}

impl Project {
    pub fn new(dir: PathBuf, files: Vec<String>) -> Self {
        Self { 
            dir, files,
            jh_pre: None, jh_post: None
        }
    }

    pub fn get_dir(&self) -> &Path {
        self.dir.as_path()
    }

    pub fn get_files(&self) -> &[String] {
        &self.files
    }

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
