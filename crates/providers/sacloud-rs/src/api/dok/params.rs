use serde::{Serialize, Deserialize};

/// Request for POST /registries/
#[derive(Serialize, Default)]
pub struct PostRegistries {
    hostname: String,
    username: String,
    password: String,
}

impl PostRegistries {
    pub fn hostname(mut self, hostname: String) -> Self { self.hostname = hostname; self }
    pub fn username(mut self, username: String) -> Self { self.username = username; self }
    pub fn password(mut self, password: String) -> Self { self.password = password; self }
}

#[derive(Debug, Serialize, Deserialize,  Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    #[default]
    #[serde(rename = "v100-32gb")]
    V100,
    #[serde(rename = "h100-80gb")]
    H100GB80,
    #[serde(rename = "h100-2g.20gb")]
    H100GB20
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Http {
    pub path: String,
    pub port: u16
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Container {
    image: String,
    registry: Option<String>,
    command: Vec<String>,
    entrypoint: Vec<String>,
    // TODO: environment is mandatory parameter according to the manual
    // however
    //  - it can be ignored
    //  - sending an empty value {} or null will lead to error
    // environment: Option<HashMap<String, String>>,
    plan: Plan,
    http: Option<Http>,
    pub start_at: Option<String>,
}

impl Container {
    pub fn image(mut self, image: String) -> Self { self.image = image; self }
    pub fn registry(mut self, registry: Option<String>) -> Self { self.registry = registry; self }
    pub fn command(mut self, command: Vec<String>) -> Self { self.command = command; self }
    pub fn entrypoint(mut self, entrypoint: Vec<String>) -> Self { self.entrypoint = entrypoint; self }
    // pub fn environment(mut self, environment: Option<HashMap<String, String>>) -> Self { self.environment = environment; self }
    pub fn plan(mut self, plan: Plan) -> Self { self.plan = plan; self }
    pub fn http(mut self, http: Http) -> Self { self.http = Some(http); self }
}

/// Request for POST /tasks/
#[derive(Serialize, Default)]
pub struct PostTasks {
    name: String,
    containers: Vec<Container>,
    tags: Vec<String>
}

impl PostTasks {
    pub fn name(mut self, name: String) -> Self { self.name = name; self }
    pub fn containers(mut self, containers: Vec<Container>) -> Self { self.containers = containers; self }
    pub fn tags(mut self, tags: Vec<String>) -> Self { self.tags = tags; self }
}




