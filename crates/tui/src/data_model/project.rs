use std::path::PathBuf;

pub struct Project {
    pub dir: PathBuf,
    pub files: Vec<String>,
    /// jh for pre-processing
    pub jh_pre: Option<tokio::task::JoinHandle<()>>,
    /// jh for post-processing
    pub jh_post: Option<tokio::task::JoinHandle<()>>,
}
