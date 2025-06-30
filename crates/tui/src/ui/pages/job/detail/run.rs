use std::sync::{Arc, Mutex};
use std::time::Duration;
use ratatui::prelude::*;
use ratatui::widgets::*;
use sacloud_rs::api::dok;
use rust_client::{self, RustClient};
// use std::io::{BufRead, BufReader};
use futures_util::stream::StreamExt;
    // use futures_util::TryStreamExt;
use anyhow::{Result, anyhow};
use std::error::Error;
use crate::data_model;
use crate::ui;
use crate::utils;


pub const HELPER: &[&str] = &[
    "Launch a job", 
];

async fn launch_job_local(
    job_mgr: Arc<Mutex<data_model::job::Manager>>,
    job_id_to_cancel: Arc<Mutex<Option<usize>>>,
    proj_dir: std::path::PathBuf,
    settings_local: data_model::provider::local::Settings, 
) -> anyhow::Result<()> {
    // TODO: currently only support one job
    let job_id = 0;
    let working_dir = "/workspace";

    let volume_binds = vec![
        format!("{}:{}", proj_dir.to_str().unwrap(), working_dir)
    ];
    let docker = bollard::Docker::connect_with_socket_defaults().unwrap();
    let container_id = utils::docker::launch_container(&docker, &settings_local.docker_image, volume_binds).await?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, format!("[Local infra] exec job {job_id}, status: using image {}", settings_local.docker_image));
        job_mgr.add_log(job_id, format!("[Local infra] exec job {job_id}, status: container {container_id} created"));
        let mut job = data_model::job::Job::new(job_id);
        job.infra = data_model::job::Infra::Local;
        let _ = job_mgr.jobs.insert(job_id, job);
    }

    let exec_id = docker.create_exec(
        &container_id,
        bollard::models::ExecConfig {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(
                vec!["sh", &settings_local.script]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            ),
            working_dir: Some(working_dir.to_string()),
            ..Default::default()
        }
    ).await?
    .id;

    if let bollard::exec::StartExecResults::Attached { mut output, .. } = docker.start_exec(&exec_id, None).await? {
        loop {
            let cancel_job = {
                let job_id_to_cancel = job_id_to_cancel.lock().unwrap();
                if let Some(id_cancel) = *job_id_to_cancel {
                    id_cancel == job_id
                } else {
                    false
                }
            };

            if cancel_job {
                let scob = bollard::query_parameters::StopContainerOptionsBuilder::default();
                docker.stop_container(&container_id, Some(scob.signal("SIGINT").t(3).build())).await?;

                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log(job_id, format!("[Local infra] exec job {job_id}, container {container_id} stopped"));
                job_mgr.local_infra_cancel_job = false;
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            if let Some(Ok(msg)) = output.next().await {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log(job_id, format!("[Local infra] exec job {job_id}, output: {msg}"));
            }
        }
    } else {
        unreachable!();
    }

    let rcob = bollard::query_parameters::RemoveContainerOptionsBuilder::default();
    docker.remove_container(&container_id, Some(rcob.force(true).build())).await.unwrap();
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, format!("[Local infra] exec job {job_id}, container {container_id} removed"));
    }

    Ok(())
}

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


async fn launch_job_rust_client(
    proj: data_model::project::Project,
    mut rust_client: rust_client::RustClient,
    job_mgr: Arc<Mutex<data_model::job::Manager>>,
) -> Result<()> {
    let job_id = 0;

    let project_name = proj.get_project_name()?;

    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, "Starting RustClient job submission...".to_string());

        let mut job = data_model::job::Job::new(job_id);
        job.infra = data_model::job::Infra::RustClient(
            format!("pending-{}", job_id),
            rust_client.url.clone(),
        );
        job_mgr.jobs.insert(job_id, job);
    }

    let input_files = ["input.sh"];
    let output_files = ["output.txt"];

    let job_result = rust_client.submit_job("./input.sh", &project_name, &input_files[..], &output_files[..]).await.map_err(|e| anyhow::Error::msg(format!("submit_job failed: {}", e)))?;
    
    let task_id = job_result.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No task ID returned from RustClient"))?;

    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, "Job submitted, monitoring status...".to_string());
    }

    let task_json = loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let task_json = match rust_client.get_job(task_id).await {
            Ok(value) => value,
            Err(e) => return Err(anyhow!("RustClient error: {}", e)),
        };

        {
            let mut job_mgr = job_mgr.lock().unwrap();
            let status = task_json.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
            job_mgr.add_log_tmp(job_id, format!("RustClient task status: {}", status));
        }

        let task_status = task_json.get("status").and_then(|s| s.as_str()).unwrap_or_default();
        match task_status {
            "done" => break task_json,
            "failed" | "error" => return Err(anyhow!("Job failed with status: {}", task_status)),
            _ => continue,
        }
    };

    for &file_name in &output_files {
        {
            let mut job_mgr = job_mgr.lock().unwrap();
            job_mgr.add_log(job_id, format!("Downloading file: {}", file_name));
        }

        rust_client.get_project_files(&project_name, file_name).await
            .map_err(|e| anyhow!("Failed to download {}: {}", file_name, e))?;

        {
            let mut job_mgr = job_mgr.lock().unwrap();
            job_mgr.add_log(job_id, format!("Successfully downloaded: {}", file_name));
        }
    }

    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(job_id, "RustClient job completed successfully".to_string());
    }

    Ok(())
}

pub fn action(_states: &mut ui::states::States, store: &data_model::Store) -> anyhow::Result<()> {
    // TODO: currently only support one job
    let job_id = 0;
    {
        let job_mgr = store.job_mgr.lock().unwrap();
        if job_mgr.jobs.contains_key(&job_id) {
            return Err(anyhow::Error::msg("current job already running"));
        }
    }

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
            let job_mgr_clone = store.job_mgr.clone();
            let job_id_to_cancel = store.cancel_job_id.clone();
            let proj_dir = proj.get_dir().to_path_buf();
            let settings_local = proj_sel.get_job_settings()
                .infra_local.as_ref()
                .ok_or(anyhow::Error::msg("no settings for local servers"))?
                .clone();
            tokio::spawn(async move {
                match launch_job_local(job_mgr_clone.clone(), job_id_to_cancel, proj_dir, settings_local).await {
                    Ok(()) => (),
                    Err(e) => {
                        let mut job_mgr = job_mgr_clone.lock().unwrap();
                        job_mgr.add_log(0, format!("run job error: {e}"));
                    } 
                }
            });
        },
        Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        Settings::SakuraInternetService(_) => {
            let mut job_mgr = store.job_mgr.lock().unwrap();
            job_mgr.add_log(0, "send the job Sakura Internet DOK service ...".to_string());
            let (with_build, param_dok) = super::params::params_dok(store)?;
            if proj.get_job_settings().dok.is_some() && with_build {
                proj.get_dir().join("Dockerfile").exists().then_some(0)
                    .ok_or(anyhow::Error::msg("using DOK service with self built docker image requires a Dockerfile under the project folder"))?;
            }
            let client = store.account_mgr.create_client(&store.setting_mgr)?.clone();
            let job_mgr_clone = store.job_mgr.clone();
            tokio::spawn(async move {
                match launch_job_dok(proj, registry_sel, client, param_dok, job_mgr_clone.clone(), with_build).await {
                    Ok(()) => (),
                    Err(e) => {
                        let mut job_mgr = job_mgr_clone.lock().unwrap();
                        job_mgr.add_log(job_id, format!("[Local infra] job {} exits with error {}", job_id, e));
                    } 
                }
            });
        }
        Settings::RustClient => {
            let mut job_mgr = store.job_mgr.lock().unwrap();
            job_mgr.add_log(0, "send the job to Rust Client service ...".to_string());
            
            if proj.get_job_settings().dok.is_some() {
                return Err(anyhow::Error::msg("not DOK service"));
            }

            let job_mgr_clone = store.job_mgr.clone();
            let proj_clone = proj.clone();

            tokio::spawn(async move {
                let rust_client = match RustClient::from_env().await {
                    Ok(client) => client,
                    Err(e) => {
                        let mut job_mgr = job_mgr_clone.lock().unwrap();
                        job_mgr.add_log(0, format!("Failed to create RustClient: {}", e));
                        return;
                    }
                };

                if let Err(e) = launch_job_rust_client(proj_clone, rust_client, job_mgr_clone.clone()).await {
                    let mut job_mgr = job_mgr_clone.lock().unwrap();
                    job_mgr.add_log(0, format!("RustClient job failed: {}", e));
                }
            });
        }

    };

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
