use sacloud_rs::api::dok;

use crate::data_model;

// pub struct ParametersDok {
//     pub image_name: String, 
//     pub registry: data_model::registry::Registry,
//     pub client: sacloud_rs::Client,
//     pub plan: dok::params::Plan,
// }

pub fn params_dok(store: &data_model::Store) -> anyhow::Result<dok::params::Container> {
    use data_model::pod::Settings;

    let (proj_sel, _proj_mgr) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let image_name = proj_sel.get_docker_image_name()?;
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?;
    let registry_id = registry_sel.dok_id.as_ref()
        .ok_or(anyhow::Error::msg(format!("registry {} with username {} has not been added into DOK service", 
            registry_sel.hostname.as_str(),
            if let Some(un) = registry_sel.username.as_ref() { un } else { "" }
        )))?;
    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    let plan = match &pod_sel.settings {
        Settings::None | Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        Settings::SakuraInternetService(dok_gpu_type) => match dok_gpu_type {
            data_model::provider::sakura_internet::DokGpuType::V100 => dok::params::Plan::V100,
            data_model::provider::sakura_internet::DokGpuType::H100 => dok::params::Plan::H100GB80,
        }
    };
    let http = if let Some(dok) = proj_sel.get_job_settings().dok.as_ref() {
        let path = if let Some(http_path) = dok.http_path.as_ref() {
            http_path.to_string()
        } else { "/".to_string() };
        let port = if let Some(http_port) = dok.http_port.as_ref() {
            *http_port
        } else { 80 };
        sacloud_rs::api::dok::params::Http { path, port }
    } else {
        sacloud_rs::api::dok::params::Http { path: "/".to_string(), port: 80 }
    };
    let container = dok::params::Container::default()
        .image(image_name.to_string())
        .registry(Some(registry_id.to_string()))
        .command(vec![])
        .entrypoint(vec![])
        .plan(plan)
        .http(http);

    // let params_dok = ParametersDok { image_name: format!("{}/{image_name}", registry_sel.hostname), registry: registry_sel.to_owned(), client, plan };

    Ok(container)
}


