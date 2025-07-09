use std::{fs, path::PathBuf};
use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::{constants, utils};

#[derive(Debug, Serialize, Deserialize)]
struct DataFile {
    account_id_sel: Option<String>,
    registry_id_sel: Option<String>,

    #[serde(default)]
    rust_client: Option<super::provider::rust_client::RustClientSettings>,
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
    pub account_id_sel: Option<String>,
    pub registry_id_sel: Option<String>,
    pub rust_client: Option<super::provider::rust_client::RustClientSettings>,
}

impl Manager {
    fn data_filepath() -> anyhow::Result<PathBuf> {
        let data_dir = utils::dirs::data_dir();
        let fp = data_dir.join(constants::FILENAME_SETTINGS);
        Ok(fp)
    }

    pub fn load() -> anyhow::Result<Self> {
        let filepath = Self::data_filepath()?; 
        if !filepath.exists() {
            std::fs::File::create(&filepath)?;
        }

        let content = fs::read_to_string(&filepath)?;
        let df = DataFile::new(&content)?;
        let s = Self {
            account_id_sel: df.account_id_sel,
            registry_id_sel: df.registry_id_sel,
            rust_client: df.rust_client,
        };

        Ok(s)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let df = DataFile {
            account_id_sel: self.account_id_sel.clone(),
            registry_id_sel: self.registry_id_sel.clone(),
            rust_client: self.rust_client.clone(),
        };

        let filepath = Self::data_filepath()?; 
        let mut file = std::fs::File::create(filepath)?;
        write!(file, "{}", df.to_string()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_save() {
        let toml_str = r#""#;
        let df = DataFile::new(toml_str).unwrap();
        assert!(df.account_id_sel.is_none());
        assert_eq!(df.to_string().unwrap(), "");

        let toml_str = r#"
            account_id_sel = "12345"
        "#;
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.account_id_sel, Some("12345".to_string()));
        assert_eq!(df.to_string().unwrap(), "account_id_sel = \"12345\"\n");
    }
}

