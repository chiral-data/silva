use std::{path::{Path, PathBuf}, sync::{Arc, Mutex}};
use std::io::{Read, Write};

use futures_util::stream::StreamExt;

use crate::data_model;

const FILENAME_ENTRYPOINT: &str = "run.sh";
const FILENAME_DOCKER: &str = "Dockerfile";

fn get_project_name(proj_dir: &Path) -> anyhow::Result<String> {
    let proj_name = proj_dir.file_name()
        .ok_or(anyhow::Error::msg("no file name for project "))?
        .to_str()
        .ok_or(anyhow::Error::msg("osString to str error"))?;
    let proj_parent = proj_dir
        .parent()
        .map(|p| p.to_str().unwrap_or(""))
        .unwrap_or("");

    Ok(format!("{proj_parent}_{proj_name}"))
}

fn get_job_settings(proj_dir: &Path) ->anyhow::Result<data_model::job::settings::Settings> {
    let settings_filepath = proj_dir.join("settings.toml");
    let job_settings = data_model::job::settings::Settings::new_from_file(&settings_filepath)
        .map_err(|e| anyhow::Error::msg(format!("{e} no settings file {settings_filepath:?}")))?;

    Ok(job_settings)
}

pub async fn prepare_build_files(
    proj_dir: &Path,
    base_image: &str,
) -> anyhow::Result<()> {
    let job_settings = get_job_settings(proj_dir)?;
    let proj_name = get_project_name(proj_dir)?;

    // create entrypoint run.sh
    let mut entrypoint_file = std::fs::File::create(proj_dir.join(FILENAME_ENTRYPOINT))?;
    writeln!(entrypoint_file, "#!/bin/bash")?;
    writeln!(entrypoint_file, "#")?;
    writeln!(entrypoint_file)?;
    for script_file in job_settings.script_files.iter() {
        writeln!(entrypoint_file, "sh {}", script_file)?;
    }
    writeln!(entrypoint_file)?;
    for output_file in job_settings.output_files.iter() {
        writeln!(entrypoint_file, "cp {} /opt/artifact", output_file)?;
    }

    // create Docker file
    let mut docker_file = std::fs::File::create(proj_dir.join(FILENAME_DOCKER))?;
    writeln!(docker_file, "FROM {base_image}")?;
    writeln!(docker_file)?;
    writeln!(docker_file, "RUN mkdir -p /opt/{proj_name}")?;
    for input_file in job_settings.input_files.iter() {
        writeln!(docker_file, "ADD ./{input_file} /opt/{proj_name}")?;
    }
    for script_file in job_settings.script_files.iter() {
        writeln!(docker_file, "ADD ./{script_file} /opt/{proj_name}")?;
    }
    if let Some(dok) = job_settings.dok {
        if let Some(extra_build_commands) = dok.extra_build_commands {
            for cmd in extra_build_commands.iter() {
                writeln!(docker_file, "{cmd}")?; 
            }
        }
    }
    writeln!(docker_file, "ADD ./{FILENAME_ENTRYPOINT} /opt/{proj_name}")?;
    writeln!(docker_file)?;
    writeln!(docker_file, "WORKDIR /opt/{proj_name}")?;
    writeln!(docker_file, "CMD [\"sh\", \"{}\"]", FILENAME_ENTRYPOINT)?;

    Ok(())
}

pub async fn build_image(
    proj_dir: &PathBuf, 
    image_name: &str, 
    job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {
    let job_settings = get_job_settings(proj_dir)?;
    let dok = job_settings.dok.as_ref()
        .ok_or(anyhow::Error::msg("no DOK settings"))?;
    let base_image = &dok.base_image;

    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[docker] build image {image_name} started ..."));
    }

    std::env::set_current_dir(proj_dir)?;

    prepare_build_files(proj_dir, base_image).await?;
   
    // create tar file for building the image
    let filename_tar = "image.tar";
    let tar_file = tokio::fs::File::create(filename_tar)
        .await.unwrap().into_std().await;
    let mut a = tar::Builder::new(tar_file);
    a.append_path("Dockerfile")?;
    a.append_path("run.sh")?;
    for input_file in job_settings.input_files.iter() {
        a.append_path(input_file)?;
    }
    for script_file in job_settings.script_files.iter() {
        a.append_path(script_file)?;
    }

    let docker = bollard::Docker::connect_with_local_defaults()?;
    let build_image_options = bollard::image::BuildImageOptions {
        dockerfile: "Dockerfile",
        t: image_name,
        platform: "linux/amd64",
        ..Default::default()
    };
    let mut file = std::fs::File::open(filename_tar)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let mut image_build_stream = docker.build_image(build_image_options, None, Some(contents.into()));
    while let Some(msg) = image_build_stream.next().await {
        match msg {
            Ok(build_info) => {
                let id = if let Some(id) = &build_info.id {
                    id.to_string()
                } else { "".to_string() };
                if let Some(info) = build_info.stream {
                    let mut job_mgr = job_mgr.lock().unwrap();
                    job_mgr.add_log_tmp(0, format!("[docker] {id}: {info}"));
                } else if let Some(status) = build_info.status {
                    let progress = if let Some(progress) = build_info.progress {
                        progress
                    } else { "".to_string() };
                    let mut job_mgr = job_mgr.lock().unwrap();
                    job_mgr.add_log_tmp(0, format!("[docker] {id} Status: {status} {progress}"));
                } else {
                    let mut job_mgr = job_mgr.lock().unwrap();
                    job_mgr.add_log_tmp(0, format!("[docker] non handled build_info {:?}", build_info));
                }
            }
            Err(e) => return Err(anyhow::Error::msg(format!("[docker] push image error {e}")))
        }
    }
    tokio::fs::remove_file(FILENAME_DOCKER).await.unwrap();
    tokio::fs::remove_file(FILENAME_ENTRYPOINT).await.unwrap();
    tokio::fs::remove_file(filename_tar).await.unwrap();

    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[docker] [build image {image_name}] completed ..."));
        job_mgr.clear_log_tmp(&0);
    }
    
    Ok(())
}

// pub async fn push_image(image_name: &str, username: Option<String>, password: Option<String>, job_mgr: Arc<Mutex<data_model::job::Manager>>) -> anyhow::Result<()> {
pub async fn push_image(registry: data_model::registry::Registry, image_name: &str, job_mgr: Arc<Mutex<data_model::job::Manager>>) -> anyhow::Result<()> {
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[docker] push image({image_name}) started ..."));
    }

    let docker = bollard::Docker::connect_with_local_defaults().unwrap();
    let push_options = bollard::image::PushImageOptions::<&str>::default();
    let credentials = bollard::auth::DockerCredentials { // for sakuracr.jp
        username: registry.username, password: registry.password,
        ..Default::default()
    };
    let mut image_push_stream = docker.push_image(image_name, Some(push_options), Some(credentials));
    while let Some(msg) = image_push_stream.next().await {
        match msg {
            Ok(push_image_info) => {
                let raw_msg = format!("push_image_info: {push_image_info:?}");
                let status = if let Some(status) = push_image_info.status {
                    status
                } else { "".to_string() };
                let progress = if let Some(progress) = push_image_info.progress {
                    progress 
                } else { "".to_string() };
                let err = if let Some(_err) = push_image_info.error {
                    raw_msg
                } else { "".to_string() };
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log_tmp(0, format!("[docker] push image {status}, {progress}, {err}"));
            }
            Err(e) => return Err(anyhow::Error::msg(format!("push image error {e}")))
        }
    }

    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[docker] push image({image_name}) completed ..."));
        job_mgr.clear_log_tmp(&0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::env;

    use super::*;

    const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS";
    const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME";
    const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD";

    /// get parameters of the container registry of Sakura Internet
    /// which is necessary for using Sakura Internet DOK service
    fn get_sakura_container_registry() -> (String, String, String) {
        (
            env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS).unwrap()
                .to_str().unwrap()
                .to_string(),
            env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME).unwrap()
                .to_str().unwrap()
                .to_string(),
            env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD).unwrap()
                .to_str().unwrap()
                .to_string(),
        )
    }

    #[tokio::test]
    async fn test_build_and_push_image() {
        let examples_dir = Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("examples");
        let (hostname, username, password) = get_sakura_container_registry();
        let registry = data_model::registry::Registry {
            hostname, username: Some(username), password: Some(password)
        };

        let proj_dir = examples_dir.join("gromacs");
        assert!(proj_dir.exists());
        let image_name = "gromacs:test_241211_2";
        let image_name = format!("{}/{image_name}", registry.hostname);
        let job_mgr = Arc::new(Mutex::new(data_model::job::Manager::load().unwrap()));
        build_image(&proj_dir, &image_name, job_mgr.clone()).await.unwrap();
        push_image(registry, &image_name, job_mgr).await.unwrap();
    }
}
