//! Pod for computation

use std::collections::HashMap;

use serde::Deserialize;

use super::provider;

#[derive(Debug, Deserialize, Clone)]
pub enum Settings {
    // SakuraInternetServer(provider::sakura_internet::ServerSettings),
    SakuraInternetServer,
    SakuraInternetService(provider::sakura_internet::DokGpuType),
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

impl Manager {
    pub fn new() -> Self {
        // TODO: right now only hard coding
        // let pod_0 = Pod { 
        //     id: 0, 
        //     type_id: 0,
        //     settings: Settings::SakuraInternetServer(provider::sakura_internet::ServerSettings {
        //         server_id: "server_1".to_string(),
        //         disk_id: "disk_1".to_string(),
        //     }),
        //     name: "gpu server 0".to_string(),
        // };
        let pod_1 = Pod { 
            id: 1, 
            type_id: 1,
            settings: Settings::SakuraInternetService(provider::sakura_internet::DokGpuType::V100),
            name: "DOK service V100".to_string(),
        };
        let pod_2 = Pod {
            id: 2,
            type_id: 1,
            settings: Settings::SakuraInternetService(provider::sakura_internet::DokGpuType::H100),
            name: "DOK service H100".to_string(),
        };
        // let pod_3 = Pod {
        //     id: 3,
        //     type_id: 5,
        //     settings: Settings::SakuraInternetServer(provider::sakura_internet::ServerSettings {
        //         server_id: "server_3".to_string(),
        //         disk_id: "disk_3".to_string(),
        //     }),
        //     name: "cpu server 3".to_string()
        // };
        let pods = vec![
            // (0, pod_0), 
            (1, pod_1), 
            (2, pod_2), 
            // (3, pod_3)
        ].into_iter().collect();

        Manager { pods, pod_id_selected: None }
    }

    pub fn selected(&self) -> Option<&Pod> {
        self.pod_id_selected
            .map(|id_sel| self.pods.get(&id_sel))?
    }
}



