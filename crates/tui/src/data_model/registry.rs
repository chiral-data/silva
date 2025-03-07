//! Docker registry
//! 
//!     When selecting the Sakura Internet DOK service, a Docker registry from Sakura Internet
//!     is recommended.

use std::path::Path;
use std::{fs, path::PathBuf};
use std::io::Write;

use sacloud_rs::api::dok;
use serde::{Deserialize, Serialize};

use crate::constants;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub hostname: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub dok_id: Option<String>,
}

impl std::fmt::Display for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}   {}", self.hostname, self.username.as_deref().unwrap_or(""))
    }
}

impl Registry {
    pub fn id(&self) -> String {
        format!("{}_{}", self.hostname, self.username.as_deref().unwrap_or(""))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DataFile {
    registries: Option<Vec<Registry>>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }

    fn to_string(&self) -> anyhow::Result<String> {
        let content = toml::to_string(&self)?;
        Ok(content)
    }
}

pub struct Manager {
    data_dir: PathBuf,
    pub registries: Vec<Registry>,
}

impl Manager {
    fn data_filepath(data_dir: &Path) -> anyhow::Result<PathBuf> {
        let fp = data_dir.join(constants::FILENAME_REGISTRIES);
        Ok(fp)
    }

    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let filepath = Self::data_filepath(data_dir)?; 
        if !filepath.exists() {
            std::fs::File::create(&filepath)?;
        }

        let content = fs::read_to_string(&filepath)?;
        let df = DataFile::new(&content)?;
        let s = Self { 
            data_dir: data_dir.to_path_buf(),
            registries: df.registries.unwrap_or_default(),
        };

        Ok(s)
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let df = DataFile {
            registries: Some(self.registries.clone())
        };

        let filepath = Self::data_filepath(data_dir)?; 
        let mut file = std::fs::File::create(filepath)?;
        write!(file, "{}", df.to_string()?)?;
        Ok(())
    }

    pub async fn initialze(&mut self, account_mgr: &super::account::Manager, setting_mgr: &super::settings::Manager) -> anyhow::Result<()> {
        let client = account_mgr.create_client(setting_mgr)?; 
        let registries_dok = dok::shortcuts::get_registries(client.clone()).await?.results;
        for registry in self.registries.iter_mut() {
            if registries_dok.iter().any(
                |r| r.hostname == registry.hostname 
                    && registry.username.is_some() 
                    && *registry.username.as_ref().unwrap() == r.username
                    && registry.dok_id.is_some()
                    && *registry.dok_id.as_ref().unwrap() == r.id
            ) { continue; } else if let Some(username) = registry.username.as_ref() {
                if let Some(password) = registry.password.as_ref() {
                    let r_dok = dok::shortcuts::create_registry(client.clone(), &registry.hostname, username, password).await?;
                    registry.dok_id = Some(r_dok.id);
                }
            }
        }
        self.save()?;


        Ok(())
    }

    pub fn selected(&self, setting_mgr: &super::settings::Manager) -> Option<&Registry> {
        setting_mgr.registry_id_sel.as_ref()
            .map(|id| self.registries.iter().find(|r| r.id() == *id))?
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

