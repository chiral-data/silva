use serde::Deserialize;

// #[derive(Debug, Deserialize, Clone)]
// pub struct ServerPlan {
//     pub server_plan_id: String,
//     pub disk_plan_id: String,
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct ServerSettings {
//     pub server_id: String,
//     pub disk_id: String,
// }

#[derive(Debug, Deserialize, Clone)]
pub enum DokGpuType {
    V100,
    H100,
}

/// settings for Sakura Iternet DOK service
#[derive(Debug, Default, Deserialize)]
pub struct DokSettings {
    pub base_image: String,
    pub extra_build_commands: Option<Vec<String>>,
}

