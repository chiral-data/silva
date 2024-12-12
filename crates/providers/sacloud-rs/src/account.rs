//! Sakura Cloud Account with Auth Keys

use crate::Client;

#[derive(Clone)]
pub struct Account {
    name: String,
    access_token: String,
    access_token_secrate: Option<String>,
}

impl Account {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn client(&self) -> Client {
        Client::new(self.access_token.clone(), self.access_token_secrate.clone())
    }
}
