use std::{collections::HashMap, path::PathBuf, sync::Arc};
use std::io::{Read, Write};

use futures_util::stream::StreamExt;
use tokio::sync::Mutex;

use crate::data_model;

pub async fn build_image(
    proj_dir: &PathBuf, 
    image_name: &str, 
    base_image: &str,
    _job_logs: Arc<Mutex<HashMap<String, Vec<String>>>>
) -> anyhow::Result<()> {
    std::env::set_current_dir(proj_dir)?;

    let proj_name = proj_dir.file_name()
        .ok_or(anyhow::Error::msg("no file name for project "))?
        .to_str()
        .ok_or(anyhow::Error::msg("osString to str error"))?;

    let settings_filepath = proj_dir.join("settings.toml");
    let proj_settings = data_model::job::settings::Settings::new_from_file(&settings_filepath)
        .map_err(|e| anyhow::Error::msg(format!("{e} no settings file settings.toml")))?;

    // create entrypoint run.sh
    let filename_entrypoint = "run.sh";
    let mut entrypoint_file = std::fs::File::create(proj_dir.join(filename_entrypoint))?;
    writeln!(entrypoint_file, "#!/bin/bash")?;
    writeln!(entrypoint_file, "#")?;
    writeln!(entrypoint_file)?;
    for script_file in proj_settings.script_files.iter() {
        writeln!(entrypoint_file, "sh {}", script_file)?;
    }
    writeln!(entrypoint_file)?;
    for output_file in proj_settings.output_files.iter() {
        writeln!(entrypoint_file, "cp {} /opt/artifact", output_file)?;
    }

    // create Docker file
    let filename_docker = "Dockerfile";
    let mut docker_file = std::fs::File::create(proj_dir.join(filename_docker))?;
    writeln!(docker_file, "FROM {base_image}")?;
    writeln!(docker_file)?;
    writeln!(docker_file, "RUN mkdir -p /opt/{proj_name}")?;
    for input_file in proj_settings.input_files.iter() {
        writeln!(docker_file, "ADD ./{input_file} /opt/{proj_name}")?;
    }
    for script_file in proj_settings.script_files.iter() {
        writeln!(docker_file, "ADD ./{script_file} /opt/{proj_name}")?;
    }
    writeln!(docker_file, "ADD ./{filename_entrypoint} /opt/{proj_name}")?;
    writeln!(docker_file)?;
    writeln!(docker_file, "WORKDIR /opt/{proj_name}")?;
    writeln!(docker_file, "CMD [\"sh\", \"{}\"]", filename_entrypoint)?;
   
    // create tar file for building the image
    let filename_tar = "image.tar";
    let tar_file = tokio::fs::File::create(filename_tar)
        .await.unwrap().into_std().await;
    let mut a = tar::Builder::new(tar_file);
    a.append_path("Dockerfile")?;
    a.append_path("run.sh")?;
    for input_file in proj_settings.input_files.iter() {
        a.append_path(input_file)?;
    }
    for script_file in proj_settings.script_files.iter() {
        a.append_path(script_file)?;
    }

    let docker = bollard::Docker::connect_with_local_defaults().unwrap();
    let build_image_options = bollard::image::BuildImageOptions {
        dockerfile: "Dockerfile",
        t: image_name,
        platform: "linux/amd64",
        ..Default::default()
    };
    let mut file = std::fs::File::open(filename_tar).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();
    let mut image_build_stream = docker.build_image(build_image_options, None, Some(contents.into()));
    while let Some(msg) = image_build_stream.next().await {
        match msg {
            Ok(build_info) => {
                let id = if let Some(id) = &build_info.id {
                    id.to_string()
                } else { "".to_string() };
                if let Some(info) = build_info.stream {
                    println!("[{id}]{info}");
                } else if let Some(status) = build_info.status {
                    let progress = if let Some(progress) = build_info.progress {
                        progress
                    } else { "".to_string() };
                    println!("[{id}] Status: {status} {progress}");
                } else {
                    println!("non handled build_info {:?}", build_info);
                }
            }
            Err(e) => println!("get error {e}")
        }
    }
    tokio::fs::remove_file(filename_docker).await.unwrap();
    tokio::fs::remove_file(filename_entrypoint).await.unwrap();
    tokio::fs::remove_file(filename_tar).await.unwrap();
    
    Ok(())
}

pub async fn push_image(image_name: &str, username: Option<String>, password: Option<String>) -> anyhow::Result<()> {
    let docker = bollard::Docker::connect_with_local_defaults().unwrap();
    let push_options = bollard::image::PushImageOptions::<&str>::default();
    let credentials = bollard::auth::DockerCredentials { // for sakuracr.jp
        username, password,
        ..Default::default()
    };
    let mut image_push_stream = docker.push_image(image_name, Some(push_options), Some(credentials));
    while let Some(msg) = image_push_stream.next().await {
        println!("Message: {msg:?}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::envs;

    use super::*;

    #[tokio::test]
    async fn test_build_and_push_image() {
        let examples_dir = Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("examples");
        let (registry_addr, registry_username, registry_password) = envs::get_sakura_container_registry();

        let proj_dir = examples_dir.join("gromacs");
        assert!(proj_dir.exists());
        let image_name = format!("{}/gromacs:test_241208_2", registry_addr);
        let base_image = "nvcr.io/hpc/gromacs:2023.2";
        let job_logs = Arc::new(Mutex::new(HashMap::new()));
        build_image(&proj_dir, &image_name, base_image, job_logs).await.unwrap();
        push_image(&image_name, Some(registry_username), Some(registry_password)).await;
    }
}
