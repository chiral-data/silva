use std::collections::HashMap;

use sacloud_rs::{api::product::{ServerPlan, ServerPlanList}, Zone};

struct Store {
    server_plans: HashMap<Zone, ServerPlanList>
}

impl Store {
    async fn new(client: sacloud_rs::Client) -> anyhow::Result<Self> {
        // load server plans
        let mut server_plans = HashMap::new();
        for zone in [Zone::Tokyo1, Zone::Tokyo2, Zone::Ishikari1, Zone::Ishikari2].into_iter() {
            let client = client.clone().set_zone(zone);
            let spl = super::cache::load_server_plans(client).await?;
            let _ = server_plans.insert(zone, spl);
        }

        Ok(Self { server_plans })
    }

    fn query(&self, cpu_num: Option<u8>, memory_m_b: Option<u32>) -> HashMap<Zone, Vec<ServerPlan>> {
        todo!()
    }
} 



