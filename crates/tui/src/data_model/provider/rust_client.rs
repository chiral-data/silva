use dotenvy::dotenv;
use std::env;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct RustClientSettings {
    pub ftp_addr: String,
    pub ftp_port: u16,
    pub user_email: String,
    pub token_api: String,
    pub user_id: String,
}

impl RustClientSettings {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenv().ok(); // Load from `.env` file if present

        let ftp_addr = env::var("FTP_ADDR")?;
        let ftp_port = env::var("FTP_PORT")?.parse::<u16>()?;
        let user_email = env::var("USER_EMAIL")?;
        let token_api = env::var("TOKEN_API")?;
        let user_id = env::var("USER_ID")?;

        Ok(Self {
            ftp_addr,
            ftp_port,
            user_email,
            token_api,
            user_id,
        })
    }
}
