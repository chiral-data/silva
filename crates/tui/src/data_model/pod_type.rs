use std::collections::HashMap;

use serde::Deserialize;

use super::app::App;


// #[derive(Debug, Deserialize, Clone)]
// pub enum Kind {
//     // SakuraInternetServer(provider::sakura_internet::ServerPlan), 
//     SakuraInternetService,
// }

#[derive(Debug, Deserialize, Clone)]
pub struct PodType {
    pub id: usize,
    // pub provider: provider::Provider,
    // kind: Kind,
    pub name: String,
    pub descs: Vec<String>,
    pub is_service: bool,
}

impl PodType {
    pub fn is_localhost(&self) -> bool {
        self.id == 0
    }
}

pub struct Manager {
    pub pod_types: HashMap<usize, PodType>,
    pub pod_type_id_selected: Option<usize>,
    /// (app, pod type ids)
    pub for_applications: HashMap<App, Vec<usize>>
}

pub mod ids {
    pub const DOK: usize = 1;
    pub const RUST_CLIENT: usize = 3;
}

impl Manager {
    pub fn new() -> Self {
        let pt_0 = PodType { 
            id: 0, 
            name: "Localhost".to_string(),
            descs: vec![],
            is_service: false,
        };
        let pt_1 = PodType { 
            id: ids::DOK, 
            name: "Sakura Internet - DOK".to_string(),
            descs: vec![
                "Providing the best GPUs for generative AI and machine learning at low prices".to_string(),
                "High-performance GPU NVIDIA H100 now in the lineup".to_string()
            ],
            is_service: true,
        };
        let pt_2 = PodType { 
            id: ids::RUST_CLIENT,
            name: "RustClient".to_string(),
            descs: vec![
                "Experimental".to_string(),
                "Experimental".to_string()
            ],
            is_service: false, 
        };
        let pod_types = vec![
            (0, pt_0), 
            (ids::DOK, pt_1), 
            (ids::RUST_CLIENT, pt_2),
        ].into_iter().collect();

        // TODO: right now only hard coding
        let for_applications = vec![
            (App::Gromacs, vec![0, 1,3]),
            (App::OpenAIWhisper, vec![1]),
            (App::Llm, vec![1])
        ].into_iter().collect();

        Manager { pod_types, pod_type_id_selected: None, for_applications }
    }

    pub fn selected(&self) -> Option<&PodType> {
        self.pod_type_id_selected
            .map(|id_sel| self.pod_types.get(&id_sel))?
    }
}



