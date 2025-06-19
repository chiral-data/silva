//! Pod for computation

use std::collections::HashMap;

use serde::Deserialize;

use super::provider;

#[derive(Debug, Deserialize, Clone)]
pub enum Settings {
    Local,
    // SakuraInternetServer(provider::sakura_internet::ServerSettings),
    SakuraInternetServer,
    SakuraInternetService(provider::sakura_internet::DokGpuType),
    RustClient,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pod {
    pub id: usize,
    pub type_id: usize,
    pub settings: Settings,
    pub name: String,
}

pub struct Manager {
    pub pods: HashMap<usize, Pod>,
    /// id of the selected server
    pub pod_id_selected: Option<usize>,
}

pub mod ids {
    pub const LOCAL: usize = 0;
    pub const DOK_V100: usize = 1;
    pub const DOK_H100: usize = 2;
    pub const RUST_CLIENT: usize = 3;
}

impl Manager {
    pub fn new() -> Self {
        let pod_0 = Pod { 
            id: 0, 
            type_id: 0,
            settings: Settings::Local,
            name: "Localhost".to_string(),
        };
        let pod_1 = Pod { 
            id: ids::DOK_V100, 
            type_id: super::pod_type::ids::DOK,
            settings: Settings::SakuraInternetService(provider::sakura_internet::DokGpuType::V100),
            name: "DOK service V100".to_string(),
        };
        let pod_2 = Pod {
            id: ids::DOK_H100,
            type_id: super::pod_type::ids::DOK,
            settings: Settings::SakuraInternetService(provider::sakura_internet::DokGpuType::H100),
            name: "DOK service H100".to_string(),
        };
        let pod_3 = Pod {
            id: ids::RUST_CLIENT,
            type_id: super::pod_type::ids::RUST_CLIENT, // we'll define this next
            settings: Settings::RustClient,
            name: "RustClient Node".to_string(),
        };
        let pods = vec![
            (0, pod_0), 
            (ids::DOK_V100, pod_1), 
            (ids::DOK_H100, pod_2), 
            (ids::RUST_CLIENT, pod_3),
        ].into_iter().collect();

        Manager { pods, pod_id_selected: None }
    }

    pub fn selected(&self) -> Option<&Pod> {
        self.pod_id_selected
            .map(|id_sel| self.pods.get(&id_sel))?
    }
}



