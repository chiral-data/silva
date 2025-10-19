use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    pub gpu: bool,
    pub memory_gb: u32,
    pub cuda_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub long_description: String,
    pub base_image: String,
    pub registry: String,
    pub image_path: String,
    pub tags: Vec<String>,
    pub requirements: Requirements,
    pub documentation_url: String,
}

impl Application {
    pub fn full_image_name(&self) -> String {
        format!("{}{}", self.registry, self.image_path)
    }

    pub fn docker_pull_command(&self) -> String {
        format!("docker pull {}", self.full_image_name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCatalog {
    pub version: String,
    pub last_updated: String,
    pub applications: Vec<Application>,
}

impl ApplicationCatalog {
    pub fn get_by_id(&self, id: &str) -> Option<&Application> {
        self.applications.iter().find(|app| app.id == id)
    }

    pub fn filter_by_category(&self, category: &str) -> Vec<&Application> {
        self.applications
            .iter()
            .filter(|app| app.category == category)
            .collect()
    }

    pub fn filter_by_tag(&self, tag: &str) -> Vec<&Application> {
        self.applications
            .iter()
            .filter(|app| app.tags.contains(&tag.to_string()))
            .collect()
    }
}
