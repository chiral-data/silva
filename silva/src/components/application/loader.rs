use std::fs;
use std::io;
use std::path::Path;

use super::model::ApplicationCatalog;

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    JsonParse(serde_json::Error),
    Http(reqwest::Error),
}

impl From<io::Error> for LoadError {
    fn from(err: io::Error) -> Self {
        LoadError::Io(err)
    }
}

impl From<serde_json::Error> for LoadError {
    fn from(err: serde_json::Error) -> Self {
        LoadError::JsonParse(err)
    }
}

impl From<reqwest::Error> for LoadError {
    fn from(err: reqwest::Error) -> Self {
        LoadError::Http(err)
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "IO error: {err}"),
            LoadError::JsonParse(err) => write!(f, "JSON parse error: {err}"),
            LoadError::Http(err) => write!(f, "HTTP error: {err}"),
        }
    }
}

impl std::error::Error for LoadError {}

pub struct ApplicationLoader {
    file_path: String,
}

impl ApplicationLoader {
    pub fn new(file_path: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }

    /// Load applications from local JSON file
    pub fn load_from_file(&self) -> Result<ApplicationCatalog, LoadError> {
        let path = Path::new(&self.file_path);
        let content = fs::read_to_string(path)?;
        let catalog: ApplicationCatalog = serde_json::from_str(&content)?;
        Ok(catalog)
    }

    /// Load applications from a URL
    /// Example URL: https://raw.githubusercontent.com/chiral-data/container-images-silva/refs/heads/main/applications.json
    #[allow(dead_code)]
    pub async fn load_from_network(&self, url: &str) -> Result<ApplicationCatalog, LoadError> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let catalog: ApplicationCatalog = response.json().await?;
        Ok(catalog)
    }

    /// Load with fallback: try network first, then local file
    #[allow(dead_code)]
    pub async fn load_with_fallback(
        &self,
        network_url: Option<&str>,
    ) -> Result<ApplicationCatalog, LoadError> {
        if let Some(url) = network_url {
            match self.load_from_network(url).await {
                Ok(catalog) => return Ok(catalog),
                Err(_) => {
                    // Network failed, try local file
                }
            }
        }

        // Fall back to local file
        self.load_from_file()
    }
}

impl Default for ApplicationLoader {
    fn default() -> Self {
        Self::new("applications.json")
    }
}
