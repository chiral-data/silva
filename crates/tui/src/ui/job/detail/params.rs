use std::path::PathBuf;

use sacloud_rs::api::dok;

use crate::data_model;

pub struct ParametersDok {
    pub image_name: String, 
    pub registry: data_model::registry::Registry,
    pub client: sacloud_rs::Client,
    pub registry_dok: dok::Registry,
    pub plan: dok::params::Plan,
}


pub fn proj_dir(store: &data_model::Store) -> anyhow::Result<PathBuf> {
    let proj_dir = store.proj_selected.as_ref()
        .ok_or(anyhow::Error::msg("no project selected"))?;
    Ok(proj_dir.to_owned())
}


pub fn params_dok(store: &data_model::Store) -> anyhow::Result<ParametersDok> {
    let proj_dir = proj_dir(store)?;
    let proj_name = data_model::job::Job::get_project_name(&proj_dir)?;
    let image_name = format!("{proj_name}:latest").to_lowercase();
    let client = store.account_mgr.create_client(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("can not create cloud client"))?;
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?;
    let registry_dok = registry_sel.find_registry_dok(&store.registry_mgr.registries_dok)
        .ok_or(anyhow::Error::msg(format!("can not find registry for Sakura DOK service {:?}", store.registry_mgr.registries_dok)))?;
    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    let plan = match &pod_sel.settings {
        data_model::pod::Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        data_model::pod::Settings::SakuraInternetService(dok_gpu_type) => match dok_gpu_type {
            data_model::provider::sakura_internet::DokGpuType::V100 => dok::params::Plan::V100,
            data_model::provider::sakura_internet::DokGpuType::H100 => dok::params::Plan::H100GB80,
        }
    };
    let params_dok = ParametersDok { image_name: format!("{}/{image_name}", registry_sel.hostname), registry: registry_sel.to_owned(), client, registry_dok, plan };

    Ok(params_dok) 
}


