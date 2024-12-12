use serde::{Deserialize, Serialize};

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
struct ServerPlan {
    i_d: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Icon {
    i_d: usize,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
struct Server {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    server_plan: ServerPlan,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connected_switches: Option<Vec<String>>, // TODO: types to be confirmed
    #[serde(skip_serializing_if = "Option::is_none")]
    interface_num: Option<usize>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Params {
    server: Server,
}

impl Params {
    pub fn name<S: ToString>(mut self, s: S) -> Self {
        self.server.name = s.to_string();
        self
    }

    pub fn server_plan(mut self, id: usize) -> Self {
        self.server.server_plan = ServerPlan { i_d: id };
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WithDisk(pub Vec<usize>);

create_struct!(ParamsWithDisk, "PascalCase",
    with_disk: WithDisk
);

impl ParamsWithDisk {
    pub fn disk_ids(mut self, ids: Vec<usize>) -> Self {
        self.with_disk = WithDisk(ids);
        self
    }
}

create_struct!(ParamsDeleteServer, "PascalCase",
    force: bool
);
