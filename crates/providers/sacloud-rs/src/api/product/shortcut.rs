use crate::{api::product::ServerPlanList, Client};

use super::DiskPlanList;

pub async fn get_server_plans(client: Client) -> anyhow::Result<ServerPlanList> {
    let mut spl: ServerPlanList = client.clone().product().server().get().await?;
    if spl.total > 100 {
        let params = super::parameter::Params::default().from(0).count(spl.total);
        let params_vec: Vec<_> = params.into();
        spl = client.clone().product().server().get_with_params(&params_vec).await?;
    }

    Ok(spl)
}

pub async fn get_disk_plans(client: Client) -> anyhow::Result<DiskPlanList> {
    let mut dpl: DiskPlanList = client.clone().product().disk().get().await?;
    if dpl.total > 100 {
        let params = super::parameter::Params::default().from(0).count(dpl.total);
        let params_vec: Vec<_> = params.into();
        dpl = client.clone().product().server().get_with_params(&params_vec).await?;
    }

    Ok(dpl)
}

#[cfg(test)]
mod tests {
    use crate::Client;
    use crate::Zone;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_get_product_server() {
        let client = Client::default().set_zone(Zone::Ishikari2);
        let spl = get_server_plans(client).await.unwrap();
        dbg!(spl.server_plans);
        // assert_eq!(spl.server_plans.len(), 147); // Zone::Ishikari1
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_product_disk() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        let dpl = get_disk_plans(client).await.unwrap();
        assert_eq!(dpl.disk_plans.len(), 2);
        dbg!(dpl.disk_plans);
    }
}
