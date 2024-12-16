use serde::Deserialize;

use crate::ui;

#[derive(Debug, Deserialize)]
pub struct SakuraAccount {
    name: String,
    resource_id: String,
    access_token: String,
    access_token_secret: String
}

#[derive(Debug, Deserialize)]
pub enum Account {
    Sakura(SakuraAccount), // Sakura Internet
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Account::Sakura(sa) => write!(f, "SKRIT   {}   {}", sa.resource_id, sa.name)
        }
    }
}

impl Account {
    pub fn create_client(&self) -> sacloud_rs::Client {
        match self {
            Account::Sakura(sa) => sacloud_rs::Client::new(sa.access_token.to_string(), Some(sa.access_token_secret.to_string()))
        }
    }
}

#[derive(Debug, Deserialize)]
struct DataFile {
    accounts: Vec<Account>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }
}

#[derive(Debug, Deserialize)]
pub struct Manager {
    accounts: Vec<Account>,
}

impl Manager {
    pub fn new(content: &str) -> Self {
        let accounts = match DataFile::new(content) {
            Ok(df) => df.accounts,
            Err(_e) => vec![]
        };

        Self { accounts }
    }

    pub fn get_accounts(&self) -> &Vec<Account> {
        &self.accounts
    }

    pub fn selected(&self, states: &ui::States) -> Option<&Account> {
        states.account.list.list.selected()
            .map(|index| self.accounts.get(index))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_file() {
        let toml_str = r#"
            [[accounts]]
            Sakura.name = "sa_1"
            Sakura.resource_id = "rid_1"
            Sakura.access_token = "at_1"
            Sakura.access_token_secret = "ats_1"

            [[accounts]]
            Sakura.name = "sa_2"
            Sakura.resource_id = "rid_2"
            Sakura.access_token = "at_2"
            Sakura.access_token_secret = "ats_2"
        "#;
        
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.accounts.len(), 2);
    }
}

