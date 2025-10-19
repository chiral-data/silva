use std::fs;
use std::io;
use std::path::Path;

use super::model::ApplicationCatalog;

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    JsonParse(serde_json::Error),
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

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "IO error: {err}"),
            LoadError::JsonParse(err) => write!(f, "JSON parse error: {err}"),
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

    /// Placeholder for future network fetch implementation
    /// This will be implemented when network fetching is needed
    #[allow(dead_code)]
    pub async fn load_from_network(&self, _url: &str) -> Result<ApplicationCatalog, LoadError> {
        // TODO: Implement network fetching using reqwest
        // For now, fall back to local file
        self.load_from_file()
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
