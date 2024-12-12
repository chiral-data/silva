use tokio::time::sleep;

use super::params;
use crate::api::disk;
use crate::{enums::EDiskConnection, Client};

pub async fn create(
    client: Client,
    disk_name: &str,
    disk_plan_id: usize,
    size_mb: usize,
    archive_id: usize,
    ssh_key: &str,
) -> anyhow::Result<String> {
    let disk_params = params::Disk::default()
        .name(disk_name)
        .plan(disk_plan_id)
        .connection(EDiskConnection::Virtio)
        .size_m_b(size_mb)
        .source_archive_id(archive_id);
    let create_params = params::Params::default().disk(disk_params);
    let disk_created: disk::DiskCreated = client.clone().disk()
        .set_params(&create_params)
        .unwrap().post().await?;
    // Query availability of a disk until "available:
    let disk_id = &disk_created.disk.i_d;
    loop {
        let new_disk_resp: disk::DiskQuery =
            client.clone().disk().diskid(disk_id).get().await.unwrap();
        if new_disk_resp.disk.availability == "available" {
            break;
        }
        sleep(std::time::Duration::from_secs(1)).await;
    }
    // Config a disk
    let config = params::Config::default()
        // .password("123456")
        .ssh_key(ssh_key);
    let _ = client
        .clone()
        .disk()
        .diskid(disk_id)
        .config()
        .set_params(&config)
        .unwrap()
        .put()
        .await?;

    Ok(disk_id.to_string())
}

pub async fn connect_to_server(client: Client,
    disk_id: &str,
    server_id: &str,
) -> anyhow::Result<()> {
    let _ = client.clear()
        .disk().diskid(disk_id)
        .to().server().serverid(server_id)
        .put().await?;

    Ok(())
}

pub async fn disconnect_to_server(client: Client,
    disk_id: &str,
) -> anyhow::Result<()> {
    let _ = client.clear()
        .disk().diskid(disk_id)
        .to().server()
        .delete().await;
    Ok(())
}

pub async fn remove(client: Client, disk_id: &str) -> anyhow::Result<()> {
    for _ in 0..30 {
        let _resp = client.clone().clear()
            .disk().diskid(disk_id)
            .delete().await?;
        let res: anyhow::Result<super::Disk> = client.clone().clear()
            .disk().diskid(disk_id)
            .get().await;
        if res.is_err() { // disk with this id does not exist
            break;
        }
        
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Client;
    use crate::Zone;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_create_disk_with_archive() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        create(client, "test_disk", 4, 20480, 113601993713, "ssh").await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_remove_disk() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        remove(client, "113602007815").await.unwrap();
    }
}
