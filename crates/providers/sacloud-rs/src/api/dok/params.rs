use serde::Serialize;

#[derive(Serialize,  Default)]
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

#[derive(Serialize, Default)]
pub struct Container {
    image: String,
    registry: Option<String>,
    command: Vec<String>,
    entrypoint: Vec<String>,
    // environment is mandatory parameter according to the manual
    // however
    //  - it can be ignored
    //  - sending an empty value {} or null will lead to error
    // environment: Option<HashMap<String, String>>,
    plan: Plan
}

impl Container {
    pub fn image(mut self, image: String) -> Self { self.image = image; self }
    pub fn registry(mut self, registry: Option<String>) -> Self { self.registry = registry; self }
    pub fn command(mut self, command: Vec<String>) -> Self { self.command = command; self }
    pub fn entrypoint(mut self, entrypoint: Vec<String>) -> Self { self.entrypoint = entrypoint; self }
    // pub fn environment(mut self, environment: Option<HashMap<String, String>>) -> Self { self.environment = environment; self }
    pub fn plan(mut self, plan: Plan) -> Self { self.plan = plan; self }
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

