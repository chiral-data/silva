use std::collections::HashMap;
use sacloud_rs::api::dok;
use crate::data_model;

// pub struct ParametersDok {
//     pub image_name: String, 
//     pub registry: data_model::registry::Registry,
//     pub client: sacloud_rs::Client,
//     pub plan: dok::params::Plan,
// }

pub fn params_dok(store: &data_model::Store) -> anyhow::Result<(bool, dok::params::Container)> {
    use data_model::pod::Settings;

    let (proj_sel, _proj_mgr) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?;
    let image_name_by_dirname = proj_sel.get_docker_image_url(registry_sel)?;
    let registry_id = registry_sel.dok_id.as_ref()
        .ok_or(anyhow::Error::msg(format!("registry {} with username {} has not been added into DOK service", 
            registry_sel.hostname.as_str(),
            if let Some(un) = registry_sel.username.as_ref() { un } else { "" }
        )))?;
    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    let plan = match &pod_sel.settings {
        Settings::Local | Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        Settings::SakuraInternetService(dok_gpu_type) => match dok_gpu_type {
            data_model::provider::sakura_internet::DokGpuType::V100 => dok::params::Plan::V100,
            data_model::provider::sakura_internet::DokGpuType::H100 => dok::params::Plan::H100GB80,
        }
        Settings::RustClient => {return Err(anyhow::Error::msg("RustClient is not supported in this context"));}    
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

    let dok = proj_sel.get_job_settings().dok.as_ref()
        .ok_or(anyhow::Error::msg("DOK settings are mandatory"))?;
    let image_name = dok.docker_image.clone().unwrap_or(image_name_by_dirname);
    let with_build = dok.docker_image.is_none();
    let commands = dok.commands.clone().unwrap_or_default();
    let entrypoint = dok.entrypoint.clone().unwrap_or_default();

    let mut container = dok::params::Container::new()
        .image(image_name.to_string())
        .registry(Some(registry_id.to_string()))
        .command(commands)
        .entrypoint(entrypoint)
        .plan(plan)
        .http(http);

    if let Some(envs) = dok.envs.as_ref() {
        let environment: HashMap<String, String> = envs.iter()
            .map(|name| (name.to_string(), std::env::var(name).unwrap()))
            .collect();
        container = container.environment(environment);
    }

    Ok((with_build, container))
}


