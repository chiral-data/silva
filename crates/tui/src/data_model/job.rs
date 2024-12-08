use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;


#[derive(Debug)]
pub struct Manager {
    /// job logs: <job id, log contents>
    pub logs: Arc<Mutex<HashMap<String, Vec<String>>>>
}

impl Manager {
    pub fn new() -> Self {
        let logs = Arc::new(Mutex::new(HashMap::new()));
        Self { logs }
    }
}




pub mod settings;
