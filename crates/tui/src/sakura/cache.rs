// use std::fs;
// use std::io::{Read, Write};

// use sacloud_rs::api::product::ServerPlanList;

// use crate::constants::APP_NAME;

// #[inline]
// fn get_cache_filename(client: &sacloud_rs::Client) -> String {
//     format!("sakura_{}_server_plans.json", client.get_zone())
// }


// pub async fn load_server_plans(client: sacloud_rs::Client) ->  anyhow::Result<ServerPlanList> {
//     let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME)?;
//     let cache_filename = get_cache_filename(&client);
//     let server_plan_list = match xdg_dirs.find_cache_file(&cache_filename) {
//         Some(path) => {
//             let mut cache_file = fs::File::open(path)?;
//             let mut buf = String::new();
//             let _ = cache_file.read_to_string(&mut buf)?;
//             let spl: ServerPlanList = serde_json::from_str(&buf)?;
//             spl
//         }
//         None => {
//             let server_plan_list = sacloud_rs::api::product::shortcuts::get_server_plans(client).await?;
//             let path = xdg_dirs.place_cache_file(&cache_filename)?;
//             let mut file = fs::File::create(path)?;
//             file.write_all(serde_json::to_string(&server_plan_list)?.as_bytes())?;
//             server_plan_list
//         }
//     };

//     Ok(server_plan_list)
// }

// pub fn clear_server_plans(client: sacloud_rs::Client) -> anyhow::Result<()> {
//     let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME)?;
//     let cache_filename = get_cache_filename(&client);
//     if let Some(path) = xdg_dirs.find_cache_file(cache_filename) {
//         fs::remove_file(path)?;
//     }

//     Ok(())
// }


// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_server_plan_list() {
//         let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();
//         let key_1 = std::env::var("SAKURA_KEY1").unwrap();
//         let key_2 = std::env::var("SAKURA_KEY2").ok();
//         let client = sacloud_rs::Client::new(key_1, key_2)
//             .set_zone(sacloud_rs::Zone::Ishikari1);
//         let cache_filename = get_cache_filename(&client);
//         assert!(xdg_dirs.find_cache_file(&cache_filename).is_none());

//         let spl = load_server_plans(client.clone()).await.unwrap();
//         assert!(spl.count == 147);
//         assert!(xdg_dirs.find_cache_file(&cache_filename).is_some());

//         clear_server_plans(client).unwrap();
//         assert!(xdg_dirs.find_cache_file(&cache_filename).is_none());
//     }
// }
