macro_rules! create_struct {
    ($name:ident, $rename_all:expr, $($v:ident : $t:ty),+) => {
        #[derive(Debug, Default, Serialize, Deserialize, Clone)]
        #[serde(rename_all = $rename_all)]
        pub struct $name {
            $(pub $v: $t,)+
        }
    };
}

mod enums;
pub use enums::Zone;

mod account;
pub use account::Account;
pub type AccountName = String;

mod client;
pub use client::Client;

pub mod api;

