
pub async fn create_server(client: sacloud_rs::Client) -> anyhow::Result<()> {
    let client = client.set_zone(sacloud_rs::Zone::Ishikari2);
    sacloud_rs::api::server::shortcut::create(client, "server_1", 100001001).await?;

    Ok(())
}
