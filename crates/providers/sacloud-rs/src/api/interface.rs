//! https://manual.sakura.ad.jp/cloud-api/1.1/interface/index.html
//!
//! API                                                 Parameters              Response
//! ------------------------------------------------------------------------------------
//! POST    /interface                                  -                       -
//! PUT     /interface/:interfaceid/to/switch/shared    -                       -

use serde::{Deserialize, Serialize};

pub mod parameter {
    use super::*;

    #[derive(Serialize, Default)]
    #[serde(rename_all = "PascalCase")]
    struct Server {
        #[serde(skip_serializing_if = "Option::is_none")]
        i_d: Option<String>,
    }

    #[derive(Serialize, Default)]
    #[serde(rename_all = "PascalCase")]
    struct Interface {
        #[serde(skip_serializing_if = "Option::is_none")]
        server: Option<Server>,
    }

    #[derive(Serialize, Default)]
    #[serde(rename_all = "PascalCase")]
    pub struct Params {
        #[serde(skip_serializing_if = "Option::is_none")]
        interface: Option<Interface>,
    }

    impl Params {
        pub fn server_id<S: ToString>(mut self, id: S) -> Self {
            let server = Server {
                i_d: Some(id.to_string()),
            };
            let interface = Interface {
                server: Some(server),
            };
            self.interface.replace(interface);
            self
        }

        // pub fn config(mut self, config: Config) -> Self {
        //     self.config.replace(config);
        //     self
        // }
    }
}

create_struct!(Interface, "PascalCase",
    i_d: String,
    i_p_address: Option<String>
);

create_struct!(InterfaceCreated, "PascalCase",
    interface: Interface,
    success: bool
);
