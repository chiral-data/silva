//! Docker registry
//! 
//!     When selecting the Sakura Internet DOK service, a Docker registry from Sakura Internet
//!     is recommended.

use std::path::PathBuf;
use std::io::Write;

use serde::Deserialize;

use crate::{constants, utils};

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub addr: String,
    pub username: Option<String>,
    pub password: Option<String>
}

#[derive(Debug, Deserialize)]
struct DataFile {
    registries: Option<Vec<Registry>>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }
}

pub struct Manager {
    pub registries: Vec<Registry>
}

impl Manager {
    fn data_filepath() -> anyhow::Result<PathBuf> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(constants::APP_NAME)?;
        let fp = xdg_dirs.get_data_home().join(constants::FILENAME_REGISTRIES);
        Ok(fp)
    }

    pub fn load() -> anyhow::Result<Self> {
        let filepath = Self::data_filepath()?; 
        if !filepath.exists() {
            std::fs::File::create(&filepath)?;
        }

        let content = utils::file::get_file_content(&filepath)?;
        let df = DataFile::new(&content)?;
        let s = Self { 
            registries: df.registries.unwrap_or_default()
        };

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_save() {
        let toml_str = r#""#;
        let df = DataFile::new(toml_str).unwrap();
        assert!(df.registries.is_none());

        let toml_str = r#"
            [[registries]]
            addr = "hub.docker.com"
        "#;
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.registries.unwrap().len(), 1); 

        let toml_str = r#"
            [[registries]]
            addr = "hub.docker.com"

            [[registries]]
            addr = "chiral.sakuracr.jp"
            username = "user"
            password = "pw"
        "#;
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.registries.unwrap().len(), 2); 
    }
}
