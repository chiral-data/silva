use serde::Deserialize;
use std::sync::{Arc, Mutex};
use futures_util::stream::StreamExt;
use crate::data_model;
use crate::utils;

/// settings for local hardware as infra  
#[derive(Debug, Default, Deserialize, Clone)]
pub struct Settings {
    pub docker_image: String,
    pub script: String,
    pub use_gpu: Option<bool>,
}

pub async fn run_single_job(
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
    let not_use_gpu = settings_local.use_gpu.is_some() && !settings_local.use_gpu.unwrap();
    let container_id = utils::docker::launch_container(&docker, &settings_local.docker_image, volume_binds, !not_use_gpu).await?;
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
            } else {
                break;
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

pub async fn run_jobs(
    job_mgr: Arc<Mutex<data_model::job::Manager>>,
    job_id_to_cancel: Arc<Mutex<Option<usize>>>,
    proj_dir: std::path::PathBuf,
    settings_vec: Vec<data_model::job::settings::Settings>,
) {
    for settings_local in settings_vec.into_iter() {
        if let Some(sl) = settings_local.infra_local {
            match run_single_job(job_mgr.clone(), job_id_to_cancel.clone(), proj_dir.clone(), sl).await {
                Ok(()) => (),
                Err(e) => {
                    let mut job_mgr = job_mgr.lock().unwrap();
                    job_mgr.add_log(0, format!("run job error: {e}"));
                } 
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_single_job() {
        let job_mgr = Arc::new(Mutex::new(data_model::job::Manager::new()));
        let job_id_to_cancel = Arc::new(Mutex::new(None));

        let temp_dir = std::env::temp_dir();
        let proj_dir = temp_dir.join("silva@test_infra_local@run_single_job");
        std::fs::create_dir_all(&proj_dir).unwrap();
        std::fs::write(proj_dir.join("run.sh"), "cat 1 > 1.txt").unwrap();

        let settings_local = Settings {
            docker_image: "ubuntu:latest".to_string(),
            script: "run.sh".to_string(),
            use_gpu: Some(false) 
        };

        assert!(!proj_dir.join("1.txt").exists());
        run_single_job(job_mgr, job_id_to_cancel, proj_dir.clone(), settings_local).await.unwrap();
        assert!(proj_dir.join("1.txt").exists());

        std::fs::remove_dir_all(&proj_dir).unwrap();
    }

    #[tokio::test]
    async fn test_run_jobs() {
        let job_mgr = Arc::new(Mutex::new(data_model::job::Manager::new()));
        let job_id_to_cancel = Arc::new(Mutex::new(None));

        let temp_dir = std::env::temp_dir();
        let proj_dir = temp_dir.join("silva@test_infra_local@run_jobs");
        std::fs::create_dir_all(&proj_dir).unwrap();
        std::fs::write(proj_dir.join("run_1.sh"), "cat 1 > 1.txt").unwrap();
        std::fs::write(proj_dir.join("run_2.sh"), "cat 2 > 2.txt").unwrap();

        let toml_str_1 = r#"
        [files]
        inputs = [
        ]
        outputs = [
        ]
        scripts = [
        ]

        [infra_local]
        docker_image = "ubuntu:latest"
        script = "run_1.sh"
        use_gpu = false
        "#;

        let toml_str_2 = r#"
        [files]
        inputs = [
        ]
        outputs = [
        ]
        scripts = [
        ]

        [infra_local]
        docker_image = "ubuntu:latest"
        script = "run_2.sh"
        use_gpu = false
        "#;

        let settings_vec = vec![toml_str_1, toml_str_2].into_iter()
            .map(|c| data_model::job::settings::Settings::new(c).unwrap())
            .collect();

        assert!(!proj_dir.join("1.txt").exists());
        assert!(!proj_dir.join("2.txt").exists());
        run_jobs(job_mgr, job_id_to_cancel, proj_dir.clone(), settings_vec).await;
        assert!(proj_dir.join("1.txt").exists());
        assert!(proj_dir.join("2.txt").exists());

        std::fs::remove_dir_all(&proj_dir).unwrap();
    }
}
