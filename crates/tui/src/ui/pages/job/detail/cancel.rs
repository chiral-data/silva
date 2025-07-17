use ratatui::prelude::*;
use ratatui::widgets::*;

use sacloud_rs::api::dok;

use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "Cancel a job", 
];

pub fn action(states: &mut ui::states::States, store: &data_model::Store) -> anyhow::Result<()> {
    let selected_job_id = states.job_states.get_selected_job_id();
    
    let job_id = match selected_job_id {
        Some(id) => id,
        None => {
            return Err(anyhow::Error::msg("No job selected. Please select a job from the list first (press Enter on a job)."));
        }
    };

    let job_mgr_clone = store.job_mgr.clone();
    let job_mgr = job_mgr_clone.lock().unwrap();
    let job_infra = job_mgr.jobs.get(&job_id)
        .ok_or(anyhow::Error::msg(format!("cannot find job {job_id}")))?
        .infra.to_owned();

    match job_infra {
        data_model::job::Infra::None => {
            // Update job status to cancelled
            let mut job_mgr = job_mgr_clone.lock().unwrap();
            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Cancelled) {
                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
            } else {
                job_mgr.add_log(job_id, "Job cancelled".to_string());
            }
        },
        data_model::job::Infra::Local => {
            let mut cancel_job_id = store.cancel_job_id.lock().unwrap();
            cancel_job_id.replace(job_id);
            
            // Update job status to cancelled
            let mut job_mgr = job_mgr_clone.lock().unwrap();
            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Cancelled) {
                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
            } else {
                job_mgr.add_log(job_id, "Job cancellation requested".to_string());
            }
        },
        data_model::job::Infra::SakuraInternetDOK(task_id, _) => {
            let job_mgr = store.job_mgr.clone();
            let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
            tokio::spawn(async move {
                match dok::shortcuts::cancel_task(client, task_id.as_str()).await {
                    Ok(task_cancelled) => {
                        let mut job_mgr = job_mgr.lock().unwrap();
                        job_mgr.add_log(job_id, format!("task {} has been canceled", task_cancelled.id));
                        if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Cancelled) {
                            job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                        }
                    },
                    Err(e) => {
                        let mut job_mgr = job_mgr.lock().unwrap();
                        job_mgr.add_log(job_id, format!("cancel task error: {e}"));
                    } 
                }
            });
        }
    }

    Ok(())
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let job_id = states.job_states.get_current_job_id();
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
