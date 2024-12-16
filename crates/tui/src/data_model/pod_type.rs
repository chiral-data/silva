use std::collections::HashMap;

use serde::Deserialize;

use super::app::App;
use super::provider;


#[derive(Debug, Deserialize, Clone)]
pub enum Kind {
    SakuraInternetServer(provider::sakura_internet::ServerPlan), 
    SakuraInternetService,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PodType {
    pub id: usize,
    pub provider: provider::Provider,
    kind: Kind,
    pub name: String,
    pub descs: Vec<String>,
    pub is_service: bool,
}


pub struct Manager {
    pub pod_types: HashMap<usize, PodType>,
    /// (app, vector of server plan id)
    pub for_applications: HashMap<App, Vec<usize>>
}

impl Manager {
    pub fn new() -> Self {
        // TODO: right now only hard coding
        // let pt_0 = PodType { 
        //     id: 0, 
        //     provider: provider::Provider::SakuraInternet,
        //     kind: Kind::SakuraInternetServer(provider::sakura_internet::ServerPlan { server_plan_id: "sp_gpu".to_string(), disk_plan_id: "dp_0".to_string() }),
        //     name: "Sakura Internet - GPU server - Storage 40GB".to_string(),
        //     descs: vec!["CPU: 4 cores", "Memory: 56GB", "GPU: NVidia Tesla V100 32GB"].into_iter().map(String::from).collect(),
        //     is_service: false,
        // };
        let pt_1 = PodType { 
            id: 1, 
            provider: provider::Provider::SakuraInternet,
            kind: Kind::SakuraInternetService,
            name: "Sakura Internet - DOK".to_string(),
            descs: vec![
                "Providing the best GPUs for generative AI and machine learning at low prices".to_string(),
                "High-performance GPU NVIDIA H100 now in the lineup".to_string()
            ],
            is_service: true,
        };
        // let pt_5 = PodType { 
        //     id: 5, 
        //     provider: provider::Provider::SakuraInternet,
        //     kind: Kind::SakuraInternetServer(provider::sakura_internet::ServerPlan { server_plan_id: "sp_cpu".to_string(), disk_plan_id: "dp_5".to_string() }),
        //     name: "Sakura Internet - CPU server - Storage 40GB".to_string(),
        //     descs: vec!["CPU: 4 cores", "Memory: 4GB"].into_iter().map(String::from).collect(),
        //     is_service: false,
        // };
        let pod_types = vec![
            // (0, pt_0), 
            (1, pt_1), 
            // (5, pt_5)
        ].into_iter().collect();

        let for_applications = vec![
            (App::Gromacs, vec![1]),
            (App::Psi4, vec![1])
        ].into_iter().collect();

        Manager { pod_types, for_applications }
    }
}



