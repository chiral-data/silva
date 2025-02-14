use sacloud_rs::api::dok;

use crate::data_model;

pub struct ParametersDok {
    pub image_name: String, 
    pub registry: data_model::registry::Registry,
    pub client: sacloud_rs::Client,
    pub plan: dok::params::Plan,
}

pub fn params_dok(store: &data_model::Store) -> anyhow::Result<ParametersDok> {
    use data_model::pod::Settings;

    let proj_sel = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj_dir = proj_sel.get_dir();
    let proj_name = data_model::job::Job::get_project_name(proj_dir)?;
    let image_name = format!("{proj_name}:latest").to_lowercase();
    let client = store.account_mgr.create_client(&store.setting_mgr)?;
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?;
    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    let plan = match &pod_sel.settings {
        Settings::None | Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        Settings::SakuraInternetService(dok_gpu_type) => match dok_gpu_type {
            data_model::provider::sakura_internet::DokGpuType::V100 => dok::params::Plan::V100,
            data_model::provider::sakura_internet::DokGpuType::H100 => dok::params::Plan::H100GB80,
        }
    };
    let params_dok = ParametersDok { image_name: format!("{}/{image_name}", registry_sel.hostname), registry: registry_sel.to_owned(), client, plan };

    Ok(params_dok)
}


