//! Cloud Service Providers
//!

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum Provider {
    SakuraInternet
}


pub mod sakura_internet;
