use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER: &[&str] = &[
    "Preview a job", 
    "e.g., generate the docker file and script file for a DOK task for preview", 
];


pub fn action(states: &mut ui::States, store: &data_model::Store) -> anyhow::Result<()> {
    let (proj_dir, job_settings, _params_dok) = super::get_job_parameters(store, states)?;
    states.info.message = "Creating job intermediate files ...".to_string();
    utils::docker::prepare_build_files(&proj_dir, &job_settings)?;
    states.info.message = format!("Preview job done for project {}", proj_dir.to_str().unwrap());
    Ok(())
}