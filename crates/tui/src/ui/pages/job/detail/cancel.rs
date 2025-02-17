use std::sync::{Arc, Mutex};
use std::time::Duration;

use ratatui::prelude::*;
use ratatui::widgets::*;

use sacloud_rs::api::dok;

use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER: &[&str] = &[
    "Cancel a job", 
];

async fn launch_job_dok(
    proj: data_model::project::Project, 
    registry: data_model::registry::Registry,
    client: sacloud_rs::Client,
    param_dok: dok::params::Container, 
    job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {

}

pub fn action(_states: &mut ui::states::States, store: &data_model::Store) -> anyhow::Result<()> {
    // TODO: currently only support one job
    let job_id = 0;
    let job_mgr = store.job_mgr.lock().unwrap();
    if job_mgr.jobs.contains_key(&job_id) {
    } else {
        return Err(anyhow::Error::msg("no running job to be cancelled"));
    }

    let (proj_sel, _) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj = proj_sel.to_owned();
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?
        .to_owned();

    if proj.get_job_settings().dok.is_some() {
        proj.get_dir().join("Dockerfile").exists().then_some(0)
            .ok_or(anyhow::Error::msg("using DOK service requires a Dockerfile under the project folder"))?;
    }
    let param_dok = super::params::params_dok(store)?;

    let job_mgr = store.job_mgr.clone();
    let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
    tokio::spawn(async move {
        match launch_job_dok(proj, registry_sel, client, param_dok, job_mgr.clone()).await {
            Ok(()) => (),
            Err(e) => {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log(0, format!("run job error: {e}"));
            } 
        }
    });

    Ok(())
}

pub fn render(f: &mut Frame, area: Rect, _states: &mut ui::states::States, store: &data_model::Store) {
    // TODO: use job id 0 for testing first
    let job_id = 0;
    let job_mgr = store.job_mgr.lock().unwrap();
    let mut logs: Vec<Line> = job_mgr.logs.get(&job_id)
        .map(|v| {
            v.iter()
            .map(|s| s.as_str())
            .map(Line::from)
            .collect()
        })
        .unwrap_or_default();
    if let Some(log_tmp) = job_mgr.logs_tmp.get(&job_id) {
        logs.push(Line::from(log_tmp.as_str()));
    }
    let job_logs = Paragraph::new(logs)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(job_logs, area);
}
