//! Sakura Internet API - Disk
//! https://manual.sakura.ad.jp/cloud-api/1.1/disk/index.html
//!
//! - [x] GET    /disk                           - ディスク一覧を取得
//! - [x] POST   /disk                           - ディスクを作成
//! - [ ] GET    /disk/:diskid                   - 該当IDのディスク情報を取得
//! - [ ] PUT    /disk/:diskid                   - ディスク情報を更新
//! - [ ] DELETE /disk/:diskid                   - 該当IDのディスクを削除
//! - [ ] PUT    /disk/:diskid/config            - ディスクの内容を書き換える
//! - [ ] GET    /disk/:diskid/monitor           - ディスクのリソースモニタ情報を取得
//! - [ ] PUT    /disk/:diskid/plan              - 該当IDのディスクのプランを変更
//! - [ ] PUT    /disk/:diskid/resize-partition  - ディスクのパーティションサイズを最適化する
//! - [ ] GET    /disk/:diskid/tag               - 該当IDのディスクに付けられたタグを取得
//! - [ ] PUT    /disk/:diskid/tag               - 該当IDのディスクに付けられるタグを変更
//! - [ ] PUT    /disk/:diskid/to/blank          - ディスクを空にする
//! - [ ] DELETE /disk/:diskid/to/server         - ディスクとサーバの接続を解除
//! - [x] PUT    /disk/:diskid/to/server/:serverid - ディスクとサーバを接続
//! - [ ] GET    /disk/tag                       - ディスクタグ一覧を取得

use serde::{Deserialize, Serialize};

// Disk plan
create_struct!(Plan, "PascalCase",
    name: String
);

create_struct!(Server, "PascalCase",
    name: String
);

create_struct!(Disk, "PascalCase",
    i_d: String,
    name: String,
    size_m_b: usize,
    created_at: String,
    plan: Plan,
    availability: String,
    server: Option<Server>
);

create_struct!(DiskList, "PascalCase",
    from: usize,
    count: usize,
    total: usize,
    disks: Vec<Disk>
);

create_struct!(DiskCreated, "PascalCase",
    success: String,
    disk: Disk
);

create_struct!(DiskQuery, "PascalCase",
    disk: Disk
);

#[cfg(test)]
mod tests {
    use ssh2::Session;

    use crate::api::server;
    use crate::Client;
    use crate::Zone;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_get_disk() {
        let client = Client::default().set_zone(Zone::Tokyo2);
        let dl: DiskList = client.clear().disk().get().await.unwrap();
        assert_eq!(dl.total, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_post_disk() {
        let zone = Zone::Tokyo2;
        let client = Client::default().set_zone(zone);
        // no disk in the zone
        let dl: DiskList = client.clone().clear().disk().get().await.unwrap();
        assert_eq!(dl.total, 0);
        // create 1 disk
        let ssh_key_path = home::home_dir().unwrap().join(".ssh").join("id_rsa.pub");
        let ssh_key = std::fs::read_to_string(&ssh_key_path).unwrap();
        let dpl: crate::api::product::DiskPlanList =
            client.clone().clear().product().disk().get().await.unwrap();
        let ubuntu_2204 = 113402076879;
        let disk_id = shortcuts::create(
            client.clone(),
            "test_disk",
            dpl.disk_plans[0].i_d,
            20480,
            ubuntu_2204,
            &ssh_key,
        )
        .await
        .unwrap();
        let dl: DiskList = client.clone().clear().disk().get().await.unwrap();
        assert_eq!(dl.total, 1);
        // create a server
        let spl: crate::api::product::ServerPlanList = client
            .clone()
            .clear()
            .product()
            .server()
            .get()
            .await
            .unwrap();
        let server_id =
            server::shortcuts::create(client.clone(), "test_server", spl.server_plans[0].i_d)
                .await
                .unwrap();
        // connect disk and server
        let _ = client
            .clone()
            .clear()
            .disk()
            .diskid(&disk_id)
            .to()
            .server()
            .serverid(&server_id)
            .put()
            .await
            .unwrap();
        // power on server
        server::shortcuts::power_on(client.clone(), &server_id)
            .await
            .unwrap();
        // test ssh
        let server_query: server::ServerQuery = client
            .clone()
            .server()
            .serverid(&server_id)
            .get()
            .await
            .unwrap();
        let ip_addr = server_query.server.interfaces[0]
            .i_p_address
            .as_ref()
            .unwrap();
        let tcp = std::net::TcpStream::connect(format!("{}:22", ip_addr)).unwrap();
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp);
        sess.handshake().unwrap();
        let home_dir = home::home_dir().unwrap();
        let private_file = home_dir.join(".ssh").join("id_rsa");
        sess.userauth_pubkey_file("ubuntu", None, private_file.as_path(), None)
            .unwrap();
        assert!(sess.authenticated());
        // shut down server
        server::shortcuts::power_off(client.clone(), &server_id).await.unwrap();
        // delete the disks
        for disk in dl.disks.iter() {
            let _res = client
                .clone()
                .clear()
                .disk()
                .diskid(&disk.i_d)
                .delete()
                .await
                .unwrap();
        }
        // delete the server & the disk
        let delte_params = crate::api::server::params::ParamsWithDisk::default()
            .disk_ids(vec![disk_id.parse::<usize>().unwrap()]);
        let _res = client
            .clone()
            .clear()
            .server()
            .serverid(&server_id)
            .set_params(&delte_params)
            .unwrap()
            .delete()
            .await
            .unwrap();
        let sl: crate::api::server::ServerList =
            client.clone().clear().server().get().await.unwrap();
        assert_eq!(sl.total, 0);
        let dl: DiskList = client.clone().clear().disk().get().await.unwrap();
        assert_eq!(dl.total, 0);
    }
}

pub mod params;
pub mod shortcuts;
