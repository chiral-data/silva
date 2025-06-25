use ratatui::prelude::*;
use ratatui::widgets::*;

use sacloud_rs::api::dok;

use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "Cancel a job", 
];

pub fn action(_states: &mut ui::states::States, store: &data_model::Store) -> anyhow::Result<()> {
    // TODO: currently only support one job
    let job_id = 0;

    let job_mgr_clone = store.job_mgr.clone();
    let job_mgr = job_mgr_clone.lock().unwrap();
    let job_infra = job_mgr.jobs.get(&job_id)
        .ok_or(anyhow::Error::msg(format!("cannot find job {job_id}")))?
        .infra.to_owned();
    let (proj_sel, _) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj = proj_sel.to_owned();

    match job_infra {
        data_model::job::Infra::None => unreachable!(),
        data_model::job::Infra::Local => {
            let mut cancel_job_id = store.cancel_job_id.lock().unwrap();
            cancel_job_id.replace(job_id);
        },
        data_model::job::Infra::SakuraInternetDOK(task_id, _) => {
            let (with_build, _param_dok) = super::params::params_dok(store)?;
            if proj.get_job_settings().dok.is_some() && with_build {
                // TODO: this requirement has to be deprecated
                proj.get_dir().join("Dockerfile").exists().then_some(0)
                    .ok_or(anyhow::Error::msg("using DOK service requires a Dockerfile under the project folder"))?;
            }

            let job_mgr = store.job_mgr.clone();
            let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
            tokio::spawn(async move {
                match dok::shortcuts::cancel_task(client, task_id.as_str()).await {
                    Ok(task_cancelled) => {
                        let mut job_mgr = job_mgr.lock().unwrap();
                        job_mgr.add_log(0, format!("task {} has been canceled", task_cancelled.id));
                    },
                    Err(e) => {
                        let mut job_mgr = job_mgr.lock().unwrap();
                        job_mgr.add_log(0, format!("cancel task error: {e}"));
                    } 
                }
            });
        },
        data_model::job::Infra::RustClient(_task_id,_url) => {
            let mut cancel_job_id = store.cancel_job_id.lock().unwrap();
            cancel_job_id.replace(job_id);
        },
    }

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
