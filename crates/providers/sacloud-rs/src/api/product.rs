use serde::{Deserialize, Serialize};

pub mod parameter;
pub mod shortcut;

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
        let params = parameter::Params::default().from(100).count(30);
        let params_vec: Vec<_> = params.into();
        let spl: ServerPlanList = client.clone().product().server()
            .get_with_params(&params_vec).await.unwrap();
        assert_eq!(spl.server_plans.len(), 30);
        // use get_with_params, from 0 count 200
        let params = parameter::Params::default().from(0).count(200);
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
