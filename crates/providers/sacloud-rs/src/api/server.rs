//! https://manual.sakura.ad.jp/cloud-api/1.1/server/index.html
//!
//! API                                 Parameters              Response
//! ---------------------------------------------------------------------------
//! GET     /server                     -                       ServerList
//! POST    /server                     Params                  ServerCreated
//! GET     /server/:serverid           -                       ServerQuery
//! GET     /server/:serverid/power
//! PUT     /server/:serverid/power
//! DELETE  /server/:serverid/power     ParamsDeleteServer

use serde::{Deserialize, Serialize};

pub mod parameter;
pub mod shortcut;

create_struct!(ServerPlan, "PascalCase",
    c_p_u: u8,
    g_p_u: u8,
    memory_m_b: u32
);

create_struct!(Disk, "PascalCase",
    i_d: String
);

create_struct!(Server, "PascalCase",
    i_d: String,
    name: String,
    host_name: String,
    created_at: String,
    server_plan: ServerPlan,
    interfaces: Vec<super::interface::Interface>,
    instance: Instance,
    disks: Vec<Disk>
);

create_struct!(ServerList, "PascalCase",
    from: usize,
    count: usize,
    total: usize,
    servers: Vec<Server>
);

create_struct!(ServerCreated, "PascalCase",
    success: bool,
    server: Server
);

create_struct!(Instance, "PascalCase",
    status: String,
    before_status: Option<String>,
    status_changed_at: Option<String>
);

create_struct!(ServerStatus, "PascalCase",
    instance: Instance
);

create_struct!(ServerQuery, "PascalCase",
    server: Server
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;
    use crate::Zone;

    #[tokio::test]
    #[ignore]
    async fn test_get_servers() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        let sl: ServerList = client.clone().server().get().await.unwrap();
        // for server in sl.servers.iter() {
        //     println!("{server:?}");
        // }
        assert_eq!(sl.total, 1);
        let server_query: ServerQuery = client
            .clone()
            .server()
            .serverid(&sl.servers[0].i_d)
            .get()
            .await
            .unwrap();
        assert!(server_query.server.interfaces[0].i_p_address.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn test_post_server() {
        let client = Client::default().set_zone(Zone::Tokyo2);
        // no server in the zone
        let sl: ServerList = client.clone().clear().server().get().await.unwrap();
        assert_eq!(sl.total, 0);
        // create 1 server
        let spl: crate::api::product::ServerPlanList = client
            .clone()
            .clear()
            .product()
            .server()
            .get()
            .await
            .unwrap();
        let create_params = parameter::Params::default()
            .name("test_server")
            .server_plan(spl.server_plans[0].i_d);
        let new_server: ServerCreated = client
            .clone()
            .clear()
            .server()
            .set_params(&create_params)
            .unwrap()
            .post()
            .await
            .unwrap();
        assert!(new_server.success);
        let sl: ServerList = client.clone().clear().server().get().await.unwrap();
        assert_eq!(sl.total, 1);
        // create interface
        let if_params =
            crate::api::interface::parameter::Params::default().server_id(new_server.server.i_d);
        let i_f: crate::api::interface::InterfaceCreated = client
            .clone()
            .clear()
            .interface()
            .set_params(&if_params)
            .unwrap()
            .post()
            .await
            .unwrap();
        assert!(i_f.success);
        // connected to shared switch
        client
            .clone()
            .clear()
            .interface()
            .interfaceid(&i_f.interface.i_d)
            .to()
            .switch()
            .shared()
            .put()
            .await
            .unwrap();
        // delete the server
        let delte_params = parameter::ParamsWithDisk::default().disk_ids(vec![]);
        let sl: ServerList = client.clone().clear().server().get().await.unwrap();
        for server in sl.servers.iter() {
            let _res = client
                .clone()
                .clear()
                .server()
                .serverid(&server.i_d)
                .set_params(&delte_params)
                .unwrap()
                .delete()
                .await
                .unwrap();
        }
        let sl: ServerList = client.clone().clear().server().get().await.unwrap();
        assert_eq!(sl.total, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_server_power() {
        let zone = Zone::Tokyo2;
        let client = Client::default().set_zone(zone);
        let spl: crate::api::product::ServerPlanList =
            client.clone().product().server().get().await.unwrap();
        // create server
        let server_id = shortcut::create(client.clone(), "test_server", spl.server_plans[0].i_d)
            .await
            .unwrap();
        let server_status: ServerStatus = client
            .clone()
            .clear()
            .server()
            .serverid(&server_id)
            .power()
            .get()
            .await
            .unwrap();
        assert!(server_status.instance.status == "down");
        // power on
        shortcut::power_on(client.clone(), &server_id)
            .await
            .unwrap();
        // power off
        shortcut::power_off(client.clone(), &server_id)
            .await
            .unwrap();
        // delete server
        shortcut::remove(client.clone(), &server_id, vec![])
            .await
            .unwrap();
    }
}
