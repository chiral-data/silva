use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sacloud_rs::api::dok;

use crate::data_model;
use data_model::job::settings::Settings as JobSettings;
use crate::ui;
use crate::utils;



pub const HELPER: &[&str] = &[
    "Launch a job", 
];


async fn launch_job(
    proj_dir: PathBuf, 
    job_settings: JobSettings,
    params_dok: super::ParametersDok,
    job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {
    // build & push the docker image
    utils::docker::build_image(&proj_dir, &job_settings, &params_dok.image_name, job_mgr.clone()).await?;
    utils::docker::push_image(params_dok.registry, &params_dok.image_name, job_mgr.clone()).await?;

    // create the task
    let client = params_dok.client.clone();
    let task_created = dok::shortcuts::create_task(client.clone(), &params_dok.image_name, &params_dok.registry_dok.id, params_dok.plan).await?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[sakura internet DOK] task {} created", task_created.id));
    }

    // check task status
    let task = loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let task = dok::shortcuts::get_task(client.clone(), &task_created.id).await?;
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log_tmp(0, format!("[sakura internet DOK] task {} status: {}", task.id, task.status));
        if task.status == "done" {
            break task;
        }
    };

    // get artifact url
    let mut count = 0;
    let af_url = loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        count += 1;
        match dok::shortcuts::get_artifact_download_url(client.clone(), &task).await {
            Ok(af_url) => break af_url,
            Err(_e) => {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log_tmp(0, 
                    format!("[sakura internet DOK] output files (artifact {}) of task {} not ready {}",
                        task.artifact.as_ref().unwrap().id, task_created.id, ".".repeat(count % 5))
                );
            }
        }
    };

    // download outputs  
    let filepath = proj_dir.join("artifact.tar.gz");
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[sakura internet DOK] downloading output files of task {}", task_created.id));
    }
    utils::file::download(&af_url.url, &filepath).await?;
    utils::file::unzip_tar_gz(&filepath, &proj_dir)?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[sakura internet DOK] downloaded output files of task {}", task_created.id));
    }
    std::fs::remove_file(&filepath)?;
    
    Ok(())
}

pub fn action(states: &mut ui::States, store: &data_model::Store) -> anyhow::Result<()> {
    let (proj_dir, job_settings, params_dok) = super::get_job_parameters(store, states)?;
    let job_mgr = store.job_mgr.clone();
    tokio::spawn(async move {
        match launch_job(proj_dir, job_settings, params_dok, job_mgr.clone()).await {
            Ok(()) => (),
            Err(e) => {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log(0, format!("run job error: {e}"));
            } 
        }
    });

    Ok(())
        
}
