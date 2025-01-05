use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER: &[&str] = &[
    "Build files", 
    "e.g., generate the docker file and script file for a DOK task for preview", 
];

pub fn action(states: &mut ui::states::States, store: &data_model::Store) -> anyhow::Result<()> {
    let proj_sel = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj_dir = &proj_sel.dir;
    let job_settings = data_model::job::Job::get_settings(proj_dir)?;
    states.info_states.message = ("Creating job intermediate files ...".to_string(), ui::layout::info::MessageLevel::Info);
    utils::docker::prepare_build_files(proj_dir, &job_settings)?;
    states.info_states.message = (format!("Job intermediate files generated for project {}", proj_dir.to_str().unwrap()), ui::layout::info::MessageLevel::Info);

    let states_current = &mut states.job_states.detail;
    states_current.tab_action = super::Tab::Files;
    Ok(())
}
