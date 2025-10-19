use std::sync::{Arc, Mutex};
use std::time::Duration;
use ratatui::prelude::*;
use ratatui::widgets::*;
use sacloud_rs::api::dok;
use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;
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
    job_id: usize,
) -> anyhow::Result<()> {

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
        if let Some(container) = task.containers.first() {
            if let Some(start_at) = &container.start_at {
                job_mgr.add_log_tmp(job_id, format!("[sakura internet DOK] task {} started at {}", task.id, start_at));
            } else {
                job_mgr.add_log_tmp(job_id, format!("[sakura internet DOK] task {} not ready for use", task.id));
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
    utils::file::extract_tar_gz(&filepath, proj.get_dir())?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, format!("[sakura internet DOK] downloaded output files of task {}", task_created.id));
    }
    std::fs::remove_file(&filepath)?;

    Ok(())
}

pub fn action(states: &mut ui::states::States, store: &mut data_model::Store) -> anyhow::Result<()> {
    let selected_job_id = states.job_states.get_selected_job_id();
    
    let job_id = match selected_job_id {
        Some(id) => id,
        None => {
            return Err(anyhow::Error::msg("No job selected. Please select a job from the list first (press Enter on a job)."));
        }
    };
    
    // Queue the job instead of executing immediately
    let job_started = {
        let mut job_mgr = store.job_mgr.lock().unwrap();
        
        // Check if job can be queued
        if let Some(job) = job_mgr.jobs.get(&job_id) {
            if job.is_running() {
                return Err(anyhow::Error::msg("Job is already running"));
            }
            if job.status == data_model::job::JobStatus::Queued {
                return Err(anyhow::Error::msg("Job is already queued"));
            }
        }
        
        // Queue the job
        job_mgr.queue_job(job_id)?;
        
        // Process the queue to potentially start the job
        let started_jobs = job_mgr.process_queue();
        
        if started_jobs.contains(&job_id) {
            states.update_info(format!("Job {} started immediately", job_id), MessageLevel::Info);
            true
        } else {
            let (queued, running, available) = job_mgr.get_queue_status();
            states.update_info(
                format!("Job {} queued (Queue: {}, Running: {}, Available: {})", 
                        job_id, queued, running, available),
                MessageLevel::Info
            );
            false
        }
    };
    
    // Execute the job if it was started (outside the lock scope)
    if job_started {
        execute_running_job(job_id, store)?;
    }
    
    Ok(())
}

fn execute_running_job(job_id: usize, store: &mut data_model::Store) -> anyhow::Result<()> {
    // Get the job and its configuration index
    let config_index = {
        let job_mgr = store.job_mgr.lock().unwrap();
        let job = job_mgr.jobs.get(&job_id)
            .ok_or(anyhow::Error::msg("Job not found"))?;
        
        if !job.is_running() {
            return Ok(()); // Job is not running, nothing to execute
        }
        
        job.config_index
            .ok_or(anyhow::Error::msg("Job has no configuration index"))?
    };

    let (proj_sel, _) = store.project_sel.as_ref()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj = proj_sel.to_owned();
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?
        .to_owned();

    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    use data_model::pod::Settings;
    match &pod_sel.settings {
        Settings::Local => {
            let settings_vec = proj_sel.get_job_settings_vec().to_owned();
            let job_settings = settings_vec.get(config_index)
                .ok_or(anyhow::Error::msg("Invalid configuration index"))?
                .clone();
            
            let job_mgr = store.job_mgr.clone();
            let job_id_to_cancel = store.cancel_job_id.clone();
            let proj_dir = proj.get_dir().to_path_buf();
            
            // Update job status to running
            {
                let mut job_mgr = job_mgr.lock().unwrap();
                if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Running) {
                    job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                }
            }
            
            tokio::spawn(async move {
                if let Some(local_settings) = job_settings.infra_local {
                    match data_model::provider::local::run_single_job(job_mgr.clone(), job_id_to_cancel, proj_dir, local_settings, job_id).await {
                        Ok(()) => {
                            let mut job_mgr = job_mgr.lock().unwrap();
                            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Completed) {
                                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                            }
                        }
                        Err(e) => {
                            let mut job_mgr = job_mgr.lock().unwrap();
                            job_mgr.add_log(job_id, format!("Job execution error: {}", e));
                            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                            }
                        }
                    }
                } else {
                    let mut job_mgr = job_mgr.lock().unwrap();
                    job_mgr.add_log(job_id, "No local infrastructure configuration found".to_string());
                    if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                        job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                    }
                }
            });
        },
        Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        Settings::SakuraInternetService(_) => {
            let settings_vec = proj_sel.get_job_settings_vec().to_owned();
            let job_settings = settings_vec.get(config_index)
                .ok_or(anyhow::Error::msg("Invalid configuration index"))?
                .clone();
            
            let mut job_mgr = store.job_mgr.lock().unwrap();
            job_mgr.add_log(job_id, "send the job Sakura Internet DOK service ...".to_string());
            
            // Update job status to running
            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Running) {
                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
            }
            
            let (with_build, param_dok) = super::params::params_dok(store)?;
            if job_settings.dok.is_some() && with_build {
                proj.get_dir().join("Dockerfile").exists().then_some(0)
                    .ok_or(anyhow::Error::msg("using DOK service with self built docker image requires a Dockerfile under the project folder"))?;
            }
            let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
            let job_mgr_clone = store.job_mgr.clone();
            tokio::spawn(async move {
                match launch_job_dok(proj, registry_sel, client, param_dok, job_mgr_clone.clone(), with_build, job_id).await {
                    Ok(()) => {
                        let mut job_mgr = job_mgr_clone.lock().unwrap();
                        if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Completed) {
                            job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                        }
                    }
                    Err(e) => {
                        let mut job_mgr = job_mgr_clone.lock().unwrap();
                        job_mgr.add_log(job_id, format!("[DOK] job {} exits with error {}", job_id, e));
                        if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                            job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                        }
                    } 
                }
            });
        }
    };

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
