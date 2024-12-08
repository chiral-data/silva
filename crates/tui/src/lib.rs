pub mod constants {
    pub const APP_NAME: &str = "silva";
    pub const FILE_ACCOUNTS: &str = "accounts.toml";
    pub const FILE_JOBS_LOCAL: &str = "jobs_local.toml";
}

mod envs;
mod utils;

mod data_model;
mod ui;
mod action;
mod entry;
pub use entry::run;

mod sakura;
mod config;

