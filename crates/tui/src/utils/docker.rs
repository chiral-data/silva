use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use futures_util::TryStreamExt;
use crate::data_model;

// const FILENAME_DOCKER: &str = "Dockerfile";

pub async fn build_image(
    _registry: &data_model::registry::Registry,
    _proj: &data_model::project::Project,
    _job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {
    todo!("rewrite according to bollard 0.19");
   //  let push_image_name = proj.get_docker_image_url(registry)?;
   //  {
   //      let mut job_mgr = job_mgr.lock().unwrap();
   //      job_mgr.add_log(0, format!("[docker] build image {push_image_name}, started ..."));
   //  }

   //  std::env::set_current_dir(proj.get_dir())?;

   //  // prepare_build_files(proj_dir, job_settings)?;
   // 
   //  // create tar file for building the image
   //  let filename_tar = "image.tar";
   //  let tar_file = tokio::fs::File::create(filename_tar)
   //      .await.unwrap().into_std().await;
   //  let mut a = tar::Builder::new(tar_file);
   //  // a.append_path(FILENAME_ENTRYPOINT)?;
   //  a.append_path(FILENAME_DOCKER)?;
   //  for input_file in proj.get_job_settings().files.inputs.iter() {
   //      a.append_path(input_file)?;
   //  }
   //  for script_file in proj.get_job_settings().files.scripts.iter() {
   //      a.append_path(script_file)?;
   //  }

   //  // add extra dirs to docker build context
   //  let job_settings = proj.get_job_settings();
   //  if let Some(dok) = &job_settings.dok {
   //      if let Some(docker_build_context_extra_dirs) = dok.docker_build_context_extra_dirs.as_ref() {
   //          for dir_str in docker_build_context_extra_dirs.iter() {
   //              let mut dir_path = PathBuf::from(dir_str);
   //              if dir_path.starts_with("~") {
   //                  dir_path = super::dirs::get_user_home()?
   //                      .join(dir_path.strip_prefix("~")?)
   //                      .canonicalize()?;
   //              };
   //              let dir_name = dir_path.file_name()
   //                  .ok_or(anyhow::Error::msg(format!("cannot get file_name for dir {dir_str}")))?;
   //              {
   //                  let mut job_mgr = job_mgr.lock().unwrap();
   //                  job_mgr.add_log(0, format!("[docker] build image {push_image_name}, add {dir_path:?} to build context as {dir_name:?} ..."));
   //              }
   //              a.append_dir_all(dir_name, &dir_path)?;
   //              a.finish()?;
   //          }
   //      }
   //  }

   //  let docker = bollard::Docker::connect_with_local_defaults()?;
   //  let build_image_options = bollard::image::BuildImageOptions {
   //      dockerfile: "Dockerfile",
   //      t: &push_image_name,
   //      platform: "linux/amd64",
   //      ..Default::default()
   //  };
   //  let mut file = std::fs::File::open(filename_tar)?;
   //  let mut contents = Vec::new();
   //  file.read_to_end(&mut contents)?;
   //  let mut image_build_stream = docker.build_image(build_image_options, None, Some(contents.into()));
   //  while let Some(msg) = image_build_stream.next().await {
   //      match msg {
   //          Ok(build_info) => {
   //              let id = if let Some(id) = &build_info.id {
   //                  id.to_string()
   //              } else { "".to_string() };
   //              if let Some(info) = build_info.stream {
   //                  let mut job_mgr = job_mgr.lock().unwrap();
   //                  job_mgr.add_log_tmp(0, format!("[docker] {id}: {info}"));
   //              } else if let Some(status) = build_info.status {
   //                  let progress = if let Some(progress) = build_info.progress {
   //                      progress
   //                  } else { "".to_string() };
   //                  let mut job_mgr = job_mgr.lock().unwrap();
   //                  job_mgr.add_log_tmp(0, format!("[docker] {id} Status: {status} {progress}"));
   //              } else {
   //                  let mut job_mgr = job_mgr.lock().unwrap();
   //                  job_mgr.add_log_tmp(0, format!("[docker] non handled build_info {:?}", build_info));
   //              }
   //          }
   //          Err(e) => return Err(anyhow::Error::msg(format!("[docker] build image error {e}")))
   //      }
   //  }
   //  tokio::fs::remove_file(filename_tar).await.unwrap();

   //  {
   //      let mut job_mgr = job_mgr.lock().unwrap();
   //      job_mgr.add_log(0, format!("[docker] build image {push_image_name} completed ..."));
   //      job_mgr.clear_log_tmp(&0);
   //  }
   //  
   //  Ok(())
}

pub async fn push_image(
    _registry: &data_model::registry::Registry,
    _proj: &data_model::project::Project,
    _job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {
    todo!("rewrite for bollard 0.19");
    // let push_image_name = proj.get_docker_image_url(registry)?;
    // {
    //     let mut job_mgr = job_mgr.lock().unwrap();
    //     job_mgr.add_log(0, format!("[docker] push image {push_image_name} started ..."));
    // }

    // let docker = bollard::Docker::connect_with_local_defaults().unwrap();
    // let push_options = bollard::image::PushImageOptions::<&str>::default();
    // let username = registry.username.to_owned();
    // let password = registry.password.to_owned();
    // let credentials = bollard::auth::DockerCredentials { // for sakuracr.jp
    //     username, password,
    //     ..Default::default()
    // };
    // let mut image_push_stream = docker.push_image(&push_image_name, Some(push_options), Some(credentials));
    // while let Some(msg) = image_push_stream.next().await {
    //     match msg {
    //         Ok(push_image_info) => {
    //             let raw_msg = format!("push_image_info: {push_image_info:?}");
    //             let status = if let Some(status) = push_image_info.status {
    //                 status
    //             } else { "".to_string() };
    //             let progress = if let Some(progress) = push_image_info.progress {
    //                 progress 
    //             } else { "".to_string() };
    //             let err = if let Some(_err) = push_image_info.error {
    //                 raw_msg
    //             } else { "".to_string() };
    //             let mut job_mgr = job_mgr.lock().unwrap();
    //             job_mgr.add_log_tmp(0, format!("[docker] push image {status}, {progress}, {err}"));
    //         }
    //         Err(e) => return Err(anyhow::Error::msg(format!("push image error {e}")))
    //     }
    // }

    // {
    //     let mut job_mgr = job_mgr.lock().unwrap();
    //     job_mgr.add_log(0, format!("[docker] push image {push_image_name} completed ..."));
    //     job_mgr.clear_log_tmp(&0);
    // }
    // Ok(())
}

fn gpu_host_config() -> bollard::models::HostConfig  {
    bollard::models::HostConfig {
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
        device_requests: Some(vec![bollard::models::DeviceRequest {
            driver: Some("".into()),
            count: Some(-1),
            device_ids: None,
            capabilities: Some(vec![vec!["gpu".into()]]),
            options: Some(HashMap::new()),
        }]),
        ..Default::default()
    }
}

pub async fn launch_container(
    docker: &bollard::Docker,
    image_name: &str,
    volume_binds: Vec<String>,
) -> anyhow::Result<String> {
    let create_image_options = bollard::query_parameters::CreateImageOptions { 
        from_image: Some(image_name.to_string()), 
        ..Default::default()
    };
    docker.create_image(Some(create_image_options), None, None).try_collect::<Vec<_>>().await.unwrap();

    let mut host_config = gpu_host_config();
    host_config.binds = Some(volume_binds);

    let container_create_body = bollard::models::ContainerCreateBody { 
        image: Some(image_name.to_string()), 
        tty: Some(true), 
        host_config: Some(host_config), 
        ..Default::default() 
    };
    let container_id = docker.create_container(
        None::<bollard::query_parameters::CreateContainerOptions>,
        container_create_body
    ).await?.id;
    docker.start_container(
        &container_id,
        None::<bollard::query_parameters::StartContainerOptions>
    ).await?;

    Ok(container_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    // const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS";
    // const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME";
    // const SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD: &str = "SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD";

    // /// get parameters of the container registry of Sakura Internet
    // /// which is necessary for using Sakura Internet DOK service
    // fn get_sakura_container_registry() -> (String, String, String) {
    //     (
    //         env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_ADDRESS).unwrap()
    //             .to_str().unwrap()
    //             .to_string(),
    //         env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_USERNAME).unwrap()
    //             .to_str().unwrap()
    //             .to_string(),
    //         env::var_os(SILVA_SAKURA_DOK_CONTAINER_REGISTRY_PASSWORD).unwrap()
    //             .to_str().unwrap()
    //             .to_string(),
    //     )
    // }

    #[tokio::test]
    async fn test_build_and_push_image() {
        todo!("rewrite according to bollard 0.19");
        // let examples_dir = Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("examples");
        // let (hostname, username, password) = get_sakura_container_registry();
        // let registry = data_model::registry::Registry {
        //     hostname, username: Some(username), password: Some(password), dok_id: None
        // };

        // let proj_dir = examples_dir.join("gromacs");
        // assert!(proj_dir.exists());
        // let job_settings = data_model::job::Job::get_settings(&proj_dir).unwrap();
        // let proj = data_model::project::Project::new(proj_dir, job_settings);
        // let job_mgr = Arc::new(Mutex::new(data_model::job::Manager::load().unwrap()));
        // build_image(&registry, &proj, job_mgr.clone()).await.unwrap();
        // push_image(&registry, &proj, job_mgr).await.unwrap();
    }

    #[tokio::test]
    async fn test_exec_ls() {
        todo!("rewrite according to bollard 0.19");
        // use bollard::container::{Config, RemoveContainerOptions};
        // use bollard::Docker;
        // use bollard::exec::{CreateExecOptions, StartExecResults};
        // use bollard::image::CreateImageOptions;
        // use futures_util::stream::StreamExt;
        // use futures_util::TryStreamExt;

        // const IMAGE: &str = "alpine:3";
        // let docker = Docker::connect_with_socket_defaults().unwrap();
        // let create_image_options = CreateImageOptions { from_image: IMAGE, ..Default::default() };
        // docker.create_image(Some(create_image_options), None, None).try_collect::<Vec<_>>().await.unwrap();

        // let alpine_config = Config { image: Some(IMAGE), tty: Some(true), ..Default::default() };
        // let id = docker.create_container::<&str, &str>(None, alpine_config).await.unwrap().id;
        // docker.start_container::<String>(&id, None).await.unwrap();

        // // non interactive
        // let create_exec_options = CreateExecOptions {
        //     attach_stdout: Some(true),
        //     attach_stderr: Some(true),
        //     cmd: Some(vec!["ls", "-l", "/"]),
        //     ..Default::default()
        // };
        // let exec = docker.create_exec(&id, create_exec_options).await.unwrap().id;
        // if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&exec, None).await.unwrap() {
        //     while let Some(Ok(msg)) = output.next().await {
        //         print!("{msg}");
        //     }
        // } else {
        //     unreachable!();
        // }

        // let remove_container_options = RemoveContainerOptions {
        //     force: true,
        //     ..Default::default()
        // };
        // docker.remove_container(&id, Some(remove_container_options)).await.unwrap();
    }

    #[tokio::test]
    async fn test_exec_gromacs() {
        use bollard::Docker;
        use bollard::exec::{CreateExecOptions, StartExecResults};
        use futures_util::stream::StreamExt;

        const IMAGE: &str = "nvcr.io/hpc/gromacs:2023.2";
        let binds = vec![
            format!(
                "{}:{}",
                "/home/qw/Downloads",
                "/mnt/test"
        )];
        let docker = Docker::connect_with_socket_defaults().unwrap();
        let container_id = launch_container(&docker, IMAGE, binds).await.unwrap();
        println!("container {container_id} created");

        // non interactive
        let create_exec_options = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["/usr/local/gromacs/avx2_256/bin/gmx", "--version"]),
            ..Default::default()
        };
        let exec = docker.create_exec(&container_id, create_exec_options).await.unwrap().id;
        if let StartExecResults::Attached { mut output, .. } = docker.start_exec(&exec, None).await.unwrap() {
            while let Some(Ok(msg)) = output.next().await {
                println!("{msg}");
            }
        } else {
            unreachable!();
        }

        let rcob = bollard::query_parameters::RemoveContainerOptionsBuilder::default();
        docker.remove_container(&container_id, Some(rcob.force(true).build())).await.unwrap();
        println!("container {container_id} removed");
    }

}
