use super::{parameter, ServerCreated, ServerQuery, ServerStatus, ServerList};
use crate::api::interface;
use crate::Client;
use tokio::time::sleep;

/// create server
pub async fn create(client: Client, server_name: &str, server_plan_id: usize) -> anyhow::Result<String> {
    let create_params = parameter::Params::default()
        .name(server_name)
        .server_plan(server_plan_id);
    let new_server: ServerCreated = client
        .clone()
        .server()
        .set_params(&create_params)
        .unwrap()
        .post()
        .await?;
    let if_params = interface::parameter::Params::default().server_id(&new_server.server.i_d);
    let interface: interface::InterfaceCreated = client
        .clone()
        .interface()
        .set_params(&if_params)
        .unwrap()
        .post()
        .await?;
    let _res = client
        .clone()
        .interface()
        .interfaceid(&interface.interface.i_d)
        .to()
        .switch()
        .shared()
        .put()
        .await?;

    Ok(new_server.server.i_d)
}

#[inline]
async fn remove_method_1(client: Client, server_id: &str, disk_ids: Vec<String>) -> anyhow::Result<()> {
    let disk_ids = disk_ids.iter().map(|id_str| id_str.parse().unwrap()).collect();
    let delete_params = parameter::ParamsWithDisk::default().disk_ids(disk_ids);
    let _resp = client.clone().server().serverid(server_id)
        .set_params(&delete_params).unwrap()
        .delete().await?;
    Ok(())
}

/// not tested yet
#[allow(unused)]
#[inline]
async fn remove_method_2(client: Client, server_id: &str, disk_ids: Vec<String>) -> anyhow::Result<()> {
    let _resp = client.clone().server().serverid(server_id)
        .delete().await?;
    for disk_id in disk_ids.into_iter() {
        crate::api::disk::shortcut::remove(client.clone(), &disk_id).await?;
    }
   Ok(())
}

pub async fn remove(client: Client, server_id: &str, disk_ids: Vec<String>) -> anyhow::Result<()> {
    remove_method_1(client, server_id, disk_ids).await
}

pub async fn power_on(client: Client, server_id: &str) -> anyhow::Result<()> {
    let mut try_times = 0;
    loop {
        let _resp = client.clone()
            .server().serverid(server_id)
            .power().put().await?;
        // to be confirmed: the status was set as "up" immediately, however ssh connnection might be ready after a while
        let server_status: ServerStatus = client.clone()
            .server().serverid(server_id)
            .power().get().await?;
        if server_status.instance.status == "up" {
            break;
        }
        sleep(std::time::Duration::from_secs(1)).await;
        try_times += 1;
        if try_times > 30 {
            return Err(anyhow::Error::msg(
                "try to power on server 30 times ... failed",
            ));
        }
    }
    Ok(())
}

pub async fn power_off(client: Client, server_id: &str) -> anyhow::Result<()> {
    let mut try_times = 0;
    loop {
        if try_times < 5 {
            let _res = client.clone()
                .server().serverid(server_id)
                .power().delete().await.unwrap();
        } else {
            let params = parameter::ParamsDeleteServer { force: true };
            let _res = client
                .clone()
                .server()
                .serverid(server_id)
                .power()
                .set_params(&params)
                .unwrap()
                .delete()
                .await
                .unwrap();
        }
        let server_status: ServerStatus = client
            .clone()
            .server()
            .serverid(server_id)
            .power()
            .get()
            .await
            .unwrap();
        if server_status.instance.status == "down" {
            break;
        }
        sleep(std::time::Duration::from_secs(1)).await;
        try_times += 1;
        if try_times > 30 {
            return Err(anyhow::Error::msg("try to power off 30 times ... failed"));
        }
    }

    Ok(())
}

pub async fn get_server_ip(client: Client, server_id: &str) -> anyhow::Result<Option<String>> {
    let server_query: ServerQuery = client
        .server().serverid(server_id)
        .get().await?;
    let ip = server_query.server.interfaces.first()
        .and_then(|interface| interface.i_p_address.clone());
    Ok(ip)
}

/// return a vector of (server id, disk ids)
pub async fn get_list_of_servers(client: Client) -> anyhow::Result<Vec<(String, Vec<String>)>> {
    let server_list: ServerList = client
        .server().get().await?;
    let server_ids = server_list.servers.into_iter()
        .map(|server| (server.i_d, server.disks.into_iter().map(|disk| disk.i_d).collect()))
        .collect();

    Ok(server_ids)
}


#[cfg(test)]
mod tests {
    use crate::Client;
    use crate::Zone;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_gpu_server_create_and_remove() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        let disk_id = crate::api::disk::shortcut::create(client.clone(), "test_disk", 4, 20480, 113601993713, "ssh").await.unwrap();
        dbg!("disk created", &disk_id);
        let server_id = create(client.clone(), "test_gpu_server", 201056004).await.unwrap();
        dbg!("server created", &server_id);
        crate::api::disk::shortcut::connect_to_server(client.clone(), &disk_id, &server_id).await.unwrap();
        // sleep(std::time::Duration::from_secs(5)).await;
        // crate::api::disk::shortcut::remove(client.clone(), &disk_id).await.unwrap();
        // remove(client.clone(), server_id, vec!["113602013954".to_string()]).await.unwrap();
        // crate::api::disk::shortcut::disconnect_to_server(client.clone(), &disk_id).await.unwrap();
        // crate::api::disk::shortcut::remove(client.clone(), &disk_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete_disk() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        let disk_id = "113602017877";
        let server_id = "113602017893";
        remove(client.clone(), server_id, vec![]).await.unwrap();
        crate::api::disk::shortcut::remove(client.clone(), disk_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_ip() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        let server_id = "113602110096";
        let ip = get_server_ip(client.clone(), server_id).await.unwrap();
        dbg!(ip);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_of_servers() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        let server_ids = get_list_of_servers(client).await.unwrap();
        dbg!(server_ids);
    }
}
