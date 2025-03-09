use std::{fs, path::PathBuf};
use std::io::Write;

use serde::Deserialize;

use crate::{constants, utils};

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
    pub fn id(&self) -> &str {
        match self {
            Account::Sakura(sa) => &sa.resource_id
        }
    }

    // pub fn create_client(&self) -> sacloud_rs::Client {
    //     match self {
    //         Account::Sakura(sa) => sacloud_rs::Client::new(sa.access_token.to_string(), Some(sa.access_token_secret.to_string()))
    //     }
    // }
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

const TEMPORY_CONTENT: &str = r#"[[accounts]]
Sakura.name = ""
Sakura.resource_id = ""
Sakura.access_token = ""
Sakura.access_token_secret = ""
"#;

#[derive(Debug)]
pub struct Manager {
    pub accounts: Vec<Account>,
}

impl Manager {
    pub fn data_filepath() -> anyhow::Result<PathBuf> {
        let data_dir = utils::dirs::data_dir();
        let fp = data_dir.join(constants::FILENAME_ACCOUNTS);
        Ok(fp)
    }

    pub fn load() -> anyhow::Result<Self> {
        let filepath = Self::data_filepath()?;
        let mut accounts = if filepath.exists() {
            let content = fs::read_to_string(&filepath)?;
            let df = DataFile::new(&content)?;
            df.accounts
        } else {
            let data_dir = utils::dirs::data_dir();
            // create a temporary file for user
            let fp = data_dir.join(format!("{}.tmp", constants::FILENAME_ACCOUNTS));
            let mut file = std::fs::File::create(fp)?;
            writeln!(&mut file, "{}", TEMPORY_CONTENT)?;
            vec![]
        };

        if let Some(sakura_access_token) = std::env::var_os(constants::SILVA_SAKURA_ACCESS_TOKEN) {
            if let Some(sakura_access_token_secrete) = std::env::var_os(constants::SILVA_SAKURA_ACCESS_TOKEN_SECRET) {
                if let Some(sakura_resource_id) = std::env::var_os(constants::SILVA_SAKURA_RESOURCE_ID) {
                    let account = Account::Sakura(SakuraAccount { 
                        name: "from_terminal_rc".to_string(), 
                        resource_id: sakura_resource_id.to_str().unwrap().to_string(),
                        access_token: sakura_access_token.to_str().unwrap().to_string(),
                        access_token_secret: sakura_access_token_secrete.to_str().unwrap().to_string()
                    });
                    accounts.push(account);
                }
            }
        }

        let s = Self { accounts };
        Ok(s)
    }

    pub fn selected(&self, setting_mgr: &super::settings::Manager) -> anyhow::Result<&Account> {
        let account_id = setting_mgr.account_id_sel.as_ref()
            .ok_or(anyhow::Error::msg("no account selected, select an account from the Setting Page"))?;
        let account = self.accounts.iter().find(|acc| acc.id() == account_id)
            .ok_or(anyhow::Error::msg(format!("can not find account with id {account_id}")))?;
        Ok(account)
    }

    pub fn create_client(&self, setting_mgr: &super::settings::Manager) -> anyhow::Result<sacloud_rs::Client> {
        let account_sel = self.selected(setting_mgr)?;
        let client = match account_sel {
            Account::Sakura(sa) => sacloud_rs::Client::new(sa.access_token.to_string(), Some(sa.access_token_secret.to_string()))
        };
        Ok(client)
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

