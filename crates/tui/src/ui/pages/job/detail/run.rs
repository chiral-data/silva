use std::sync::{Arc, Mutex};
use std::time::Duration;

use ratatui::prelude::*;
use ratatui::widgets::*;

use sacloud_rs::api::dok;

use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER: &[&str] = &[
    "Launch a job", 
];

async fn launch_job_dok(
    proj: data_model::project::Project, 
    registry: data_model::registry::Registry,
    client: sacloud_rs::Client,
    param_dok: dok::params::Container, 
    job_mgr: Arc<Mutex<data_model::job::Manager>>,
    with_build: bool,
) -> anyhow::Result<()> {
    // TODO: currently only support one job
    let job_id = 0;

    if with_build {
        // build & push the docker image
        utils::docker::build_image(&registry, &proj, job_mgr.clone()).await?;
        utils::docker::push_image(&registry, &proj, job_mgr.clone()).await?;
    } else {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, "Use docker image directly, no image will be built and pushed.".to_string());
        job_mgr.add_log(job_id, "All docker building parameters in @job.toml will be ignored".to_string());
    }

    // create the task
    let task_created = dok::shortcuts::create_task(client.clone(), param_dok).await?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, format!("[sakura internet DOK] task {} created", task_created.id));
        let mut job = data_model::job::Job::new(job_id);
        job.infra = data_model::job::Infra::SakuraInternetDOK(task_created.id.to_string(), None);
        let _ = job_mgr.jobs.insert(job_id, job);
    }

    // check task status
    let task = loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let task = dok::shortcuts::get_task(client.clone(), &task_created.id).await?;
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log_tmp(job_id, format!("[sakura internet DOK] task {} status: {}", task.id, task.status));
        if task.status == "done" {
            job_mgr.clear_log_tmp(&job_id);
            break task;
        }
        if let Some(http_uri) = task.http_uri.as_ref() {
            if let Some(job) = job_mgr.jobs.get_mut(&job_id) {
                job.infra = data_model::job::Infra::SakuraInternetDOK(task.id.to_string(), Some(http_uri.to_string()));
            }
        }
    };

    // get artifact url
    let mut count = 0;
    let af_url = loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        count += 1;
        match dok::shortcuts::get_artifact_download_url(client.clone(), &task).await {
            Ok(af_url) => {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.clear_log_tmp(&job_id);
                break af_url;
            }
            Err(_e) => {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log_tmp(job_id, 
                    format!("[sakura internet DOK] output files (artifact {}) of task {} not ready {}",
                        task.artifact.as_ref().unwrap().id, task_created.id, ".".repeat(count % 5))
                );
            }
        }
    };

    // download outputs  
    let filepath = proj.get_dir().join("artifact.tar.gz");
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, format!("[sakura internet DOK] downloading output files of task {}", task_created.id));
    }
    utils::file::download(&af_url.url, &filepath).await?;
    utils::file::unzip_tar_gz(&filepath, proj.get_dir())?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, format!("[sakura internet DOK] downloaded output files of task {}", task_created.id));
    }
    std::fs::remove_file(&filepath)?;

    Ok(())
}

pub fn action(_states: &mut ui::states::States, store: &data_model::Store) -> anyhow::Result<()> {
    // TODO: currently only support one job
    let job_id = 0;
    let job_mgr = store.job_mgr.lock().unwrap();
    if job_mgr.jobs.contains_key(&job_id) {
        return Err(anyhow::Error::msg("current job already running"));
    }

    let (proj_sel, _) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj = proj_sel.to_owned();
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?
        .to_owned();

    let (with_build, param_dok) = super::params::params_dok(store)?;
    if proj.get_job_settings().dok.is_some() && with_build {
        proj.get_dir().join("Dockerfile").exists().then_some(0)
            .ok_or(anyhow::Error::msg("using DOK service with self built docker image requires a Dockerfile under the project folder"))?;
    }
    let job_mgr = store.job_mgr.clone();
    let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
    tokio::spawn(async move {
        match launch_job_dok(proj, registry_sel, client, param_dok, job_mgr.clone(), with_build).await {
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
