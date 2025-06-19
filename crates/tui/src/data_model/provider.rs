//! Cloud Service Providers
//!

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum Provider {
    SakuraInternet,
    RustClient,
}


pub mod sakura_internet;
pub mod local;
pub mod rust_client;