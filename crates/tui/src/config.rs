// use std::{fs::File, io::Read};
// use serde::Deserialize;
// use super::sakura::config::Configuration as SakuraConfiguration;

// struct ConfigurationFile {}

// impl ConfigurationFile {
//     fn read() -> anyhow::Result<String> {
//         let xdg_dirs = xdg::BaseDirectories::with_prefix(crate::constant::APP_NAME)?;
//         let content = match xdg_dirs.find_config_file(crate::constant::CONFIG_FILENAME) {
//             Some(path) => {
//                 let mut config_file = File::open(path)?;
//                 let mut buf = String::new();
//                 let _read_size = config_file.read_to_string(&mut buf)?;
//                 buf
//             }
//             None => {
//                 let config_filepath = xdg_dirs.place_config_file(crate::constant::CONFIG_FILENAME)?;
//                 let _config_file = File::create(config_filepath)?;
//                 String::from("")
//             }
//         };

//         Ok(content)
//     }
// }
