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
    let job_mgr = store.job_mgr.lock().unwrap();
    let job = job_mgr.jobs.get(&job_id)
        .ok_or(anyhow::Error::msg(format!("cannot find job {job_id}")))?;
    let task_id = match &job.infra {
        data_model::job::Infra::SakuraInternetDOK(task_id, _) => Some(task_id.to_string()),
        _ => None
    }.ok_or(anyhow::Error::msg("not a DOK job (task)"))?;

    let (proj_sel, _) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj = proj_sel.to_owned();

    if proj.get_job_settings().dok.is_some() {
        proj.get_dir().join("Dockerfile").exists().then_some(0)
            .ok_or(anyhow::Error::msg("using DOK service requires a Dockerfile under the project folder"))?;
    }

    let job_mgr = store.job_mgr.clone();
    let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
    tokio::spawn(async move {
        match dok::shortcuts::cancel_task(client, task_id.as_str()).await {
            Ok(_task_cancelled) => (),
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
