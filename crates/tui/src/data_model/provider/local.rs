use serde::Deserialize;

/// settings for local hardware as infra  
#[derive(Debug, Default, Deserialize, Clone)]
pub struct Settings {
    pub docker_image: String,
    pub script: String
}

