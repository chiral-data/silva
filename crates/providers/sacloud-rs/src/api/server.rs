//! Sakura Internet API - Server
//! https://manual.sakura.ad.jp/cloud-api/1.1/server/index.html
//!
//! - [ ] GET    /privatehost                         - 専有ホスト契約を検索・一覧を取得
//! - [ ] POST   /privatehost                         - 専有ホスト契約を作成
//! - [ ] GET    /privatehost/:privatehostid          - 該当IDの専有ホスト契約情報を取得
//! - [ ] PUT    /privatehost/:privatehostid          - 専有ホスト契約情報を更新
//! - [ ] DELETE /privatehost/:privatehostid          - 該当IDの専有ホスト契約を削除
//! - [x] GET    /server                              - サーバを検索・一覧を取得
//! - [x] POST   /server                              - サーバを作成
//! - [ ] GET    /server/:serverid                    - 該当IDのサーバ情報を取得
//! - [x] PUT    /server/:serverid                    - サーバ情報を更新
//! - [ ] DELETE /server/:serverid                    - 該当IDのサーバを削除
//! - [ ] GET    /server/:serverid/cdrom              - 該当IDのサーバに挿入されたISOイメージの状態を取得
//! - [ ] PUT    /server/:serverid/cdrom              - 該当IDのサーバにISOイメージを挿入
//! - [ ] DELETE /server/:serverid/cdrom              - 該当IDのサーバからISOイメージを排出
//! - [ ] GET    /server/:serverid/interface          - 該当IDのサーバが備えるインタフェースを取得
//! - [ ] PUT    /server/:serverid/keyboard           - 該当IDのサーバのキーボードを押下
//! - [ ] GET    /server/:serverid/monitor            - 該当IDのサーバのリソースモニタ情報を取得
//! - [ ] PUT    /server/:serverid/mouse/:mouseindex  - 該当IDのサーバのマウスを操作
//! - [ ] PUT    /server/:serverid/plan               - 該当IDのサーバのプランを変更
//! - [x] GET    /server/:serverid/power              - 該当IDのサーバの起動状態を取得
//! - [x] PUT    /server/:serverid/power              - 該当IDのサーバを起動
//! - [x] DELETE /server/:serverid/power              - 該当IDのサーバを停止、または強制停止
//! - [ ] PUT    /server/:serverid/qemu/nmi           - nmi 情報の取得
//! - [ ] PUT    /server/:serverid/reset              - 該当IDのサーバのリセットボタンを押下
//! - [ ] GET    /server/:serverid/tag                - 該当IDのサーバに付けられたタグを取得
//! - [ ] PUT    /server/:serverid/tag                - 該当IDのサーバに付けられるタグを変更
//! - [ ] PUT    /server/:serverid/to/plan/:planid    - 該当IDのサーバのプランを変更
//! - [ ] GET    /server/:serverid/vnc/proxy          - 該当IDのサーバのVNCプロクシ情報を取得
//! - [ ] GET    /server/:serverid/vnc/size           - 該当IDのサーバの現在の画面サイズを取得
//! - [ ] GET    /server/:serverid/vnc/snapshot       - 該当IDのサーバのVNCスナップショットを取得
//! - [ ] GET    /server/tag                          - サーバタグ一覧を取得

use serde::{Deserialize, Serialize};

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
        let create_params = params::Params::default()
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
        let delte_params = params::ParamsWithDisk::default().disk_ids(vec![]);
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
        let server_id = shortcuts::create(client.clone(), "test_server", spl.server_plans[0].i_d)
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
        shortcuts::power_on(client.clone(), &server_id)
            .await
            .unwrap();
        // power off
        shortcuts::power_off(client.clone(), &server_id)
            .await
            .unwrap();
        // delete server
        shortcuts::remove(client.clone(), &server_id, vec![])
            .await
            .unwrap();
    }
}


pub mod params;
pub mod shortcuts;
