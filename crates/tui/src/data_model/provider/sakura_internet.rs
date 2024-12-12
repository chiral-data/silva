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


