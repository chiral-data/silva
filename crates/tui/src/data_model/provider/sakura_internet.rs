use sacloud_rs::api::dok;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum DokGpuType {
    V100,
    H100,
}

/// settings for Sakura Iternet DOK service
#[derive(Debug, Default, Deserialize, Clone)]
pub struct DokSettings {
    pub docker_image: Option<String>,
    pub docker_build_context_extra_dirs: Option<Vec<String>>,
    pub http_path: Option<String>,
    pub http_port: Option<u16>,
    pub plan: Option<dok::params::Plan>,
    pub commands: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    /// environmental variables
    /// only env names need to be provided, ``
    pub envs: Option<Vec<String>>,
}

