use serde::Deserialize;

/// settings for local hardware as infra  
#[derive(Debug, Default, Deserialize, Clone)]
pub struct Settings {
    pub docker_image: String,
    pub mount_volume: String,
    pub script: String
}

