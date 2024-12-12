//! Sakura Internet API - Product
//! https://manual.sakura.ad.jp/cloud-api/1.1/product/index.html
//!
//! - [x] GET    /product/disk                          - ディスクプラン一覧を取得
//! - [ ] GET    /product/disk/:diskplanid              - 該当IDのディスクプラン情報を取得
//! - [ ] GET    /product/internet                      - ルータ帯域一覧を取得
//! - [ ] GET    /product/internet/:internetplanid      - 該当IDのルータ帯域情報を取得
//! - [ ] GET    /product/license                       - ライセンス情報一覧を取得
//! - [ ] GET    /product/license/:licenseid            - 該当IDのライセンス情報を取得
//! - [ ] GET    /product/privatehost                   - 専有ホストプラン一覧を取得
//! - [ ] GET    /product/privatehost/:privatehostplanid - 該当IDの専有ホストプラン情報を取得
//! - [x] GET    /product/server                        - サーバプラン一覧を取得
//! - [ ] GET    /product/server/:serverplanid          - 該当IDのサーバプラン情報を取得
//! - [ ] GET    /public/price                          - リクエスト先ゾーンの価格表を取得


use serde::{Deserialize, Serialize};

create_struct!(ServerPlan, "PascalCase",
    index : usize,
    i_d : usize,
    name : String,
    description : String,
    c_p_u_model : String,
    c_p_u : u8,
    memory_m_b : u32,
    g_p_u: u8,
    commitment: String,
    generation: u8,
    service_class: String,
    availability: String
);

create_struct!(ServerPlanList, "PascalCase",
    from: usize,
    count: usize,
    total: usize,
    server_plans: Vec<ServerPlan>
);

create_struct!(DiskSize, "PascalCase",
    size_m_b: usize,
    display_size: usize,
    display_suffix: String,
    availability: String,
    service_class: String
);

create_struct!(DiskPlan, "PascalCase",
    index: usize,
    i_d: usize,
    storage_class: String,
    display_order: usize,
    name: String,
    description: String,
    availability: String,
    size: Vec<DiskSize>
);

create_struct!(DiskPlanList, "PascalCase",
    from: usize,
    count: usize,
    total: usize,
    disk_plans: Vec<DiskPlan>
);

#[cfg(test)]
mod tests {
    use crate::Client;
    use crate::Zone;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_get_product_server() {
        let client = Client::default().set_zone(Zone::Ishikari1);
        // use get
        let spl: ServerPlanList = client.clone().product().server()
            .get().await.unwrap();
        assert_eq!(spl.server_plans.len(), 100);
        // use get_with_params, from 100 count 30
        let params = params::Params::default().from(100).count(30);
        let params_vec: Vec<_> = params.into();
        let spl: ServerPlanList = client.clone().product().server()
            .get_with_params(&params_vec).await.unwrap();
        assert_eq!(spl.server_plans.len(), 30);
        // use get_with_params, from 0 count 200
        let params = params::Params::default().from(0).count(200);
        let params_vec: Vec<_> = params.into();
        let spl: ServerPlanList = client.clone().clear().product().server()
            .get_with_params(&params_vec).await.unwrap();
        assert_eq!(spl.server_plans.len(), 147);
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_product_disk() {
        let client = Client::default().set_zone(Zone::Tokyo2);
        let dpl: DiskPlanList = client.clear().product().disk().get().await.unwrap();
        assert_eq!(dpl.disk_plans.len(), 2);
    }
}


pub mod params;
pub mod shortcuts;
