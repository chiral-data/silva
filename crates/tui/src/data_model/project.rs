use std::path::PathBuf;

pub struct Project {
    pub dir: PathBuf,
    pub files: Vec<String>,
}
