use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum DokGpuType {
    V100,
    H100,
}

/// settings for Sakura Iternet DOK service
#[derive(Debug, Default, Deserialize, Clone)]
pub struct DokSettings {
    // pub base_image: String,
    // pub extra_build_commands: Option<Vec<String>>,
    pub http_path: Option<String>,
    pub http_port: Option<u16>,
}

