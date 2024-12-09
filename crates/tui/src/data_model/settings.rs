use std::path::PathBuf;

use serde::Deserialize;

use crate::{constants, utils};

#[derive(Debug, Deserialize)]
struct DataFile {
    account_id_sel: Option<String>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }
}

pub struct Manager {
    account_id_sel: Option<String>
}

impl Manager {
    fn data_filepath() -> anyhow::Result<PathBuf> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(constants::APP_NAME)?;
        let fp = xdg_dirs.get_data_home().join(constants::FILENAME_SETTINGS);
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
            account_id_sel: df.account_id_sel
        };

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_file() {
        let toml_str = r#""#;
        let df = DataFile::new(toml_str).unwrap();
        assert!(df.account_id_sel.is_none());

        let toml_str = r#"
            account_id_sel = "12345"
        "#;
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.account_id_sel, Some("12345".to_string()));
    }
}

