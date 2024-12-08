use std::{io::Write, path::{Path, PathBuf}};
use futures_util::stream::StreamExt;

use crate::data_model;


pub async fn build_image(proj_dir: &PathBuf, image_name: &str, base_image: &str) -> anyhow::Result<()> {
    std::env::set_current_dir(proj_dir)?;

    let proj_name = proj_dir.file_name()
        .ok_or(anyhow::Error::msg("no file name for project "))?
        .to_str()
        .ok_or(anyhow::Error::msg("osString to str error"))?;

    let settings_filepath = proj_dir.join("settings.toml");
    let proj_settings = data_model::job::settings::Settings::new_from_file(&settings_filepath)
        .map_err(|e| anyhow::Error::msg(format!("{e} no settings file settings.toml")))?;

    let filename_docker = "Dockerfile";
    let filename_tar = "image.tar";

    // create Docker file
    let mut docker_file = std::fs::File::create(proj_dir.join(filename_docker))?;
    writeln!(docker_file, "FROM {base_image}")?;
    writeln!(docker_file)?;
    writeln!(docker_file, "RUN mkdir -p /opt/{proj_name}")?;
    for input_file in proj_settings.input_files.iter() {
        writeln!(docker_file, "ADD ./{input_file} /opt/{proj_name}")?;
    }
    writeln!(docker_file)?;
    writeln!(docker_file, "WORKDIR /opt/{proj_name}")?;
    writeln!(docker_file, "ENTRYPOINT [\"sh\", \"run.sh\"]")?;
    
    // let tar_file = tokio::fs::File::create("image.tar")
    //     .await.unwrap().into_std().await;
    // let mut a = tar::Builder::new(tar_file);
    // a.append_path("Dockerfile").unwrap();
    // a.append_path("1AKI_clean.pdb").unwrap();
    // a.append_path("run.sh").unwrap();

    // let docker = Docker::connect_with_local_defaults().unwrap();
    // let build_image_options = bollard::image::BuildImageOptions {
    //     // dockerfile: dockerfile_path.to_str().unwrap(),
    //     dockerfile: "Dockerfile",
    //     t: image_name,
    //     platform: "linux/amd64",
    //     ..Default::default()
    // };
    // println!("start building");
    // let mut file = std::fs::File::open("image.tar").unwrap();
    // let mut contents = Vec::new();
    // file.read_to_end(&mut contents).unwrap();
    // let mut image_build_stream = docker.build_image(build_image_options, None, Some(contents.into()));
    // // let mut image_build_stream = docker.build_image(build_image_options, None, None);
    // while let Some(msg) = image_build_stream.next().await {
    //     // println!("Message: {msg:?}");
    //     match msg {
    //         Ok(build_info) => {
    //             let id = if let Some(id) = &build_info.id {
    //                 id.to_string()
    //             } else { "".to_string() };
    //             if let Some(info) = build_info.stream {
    //                 println!("[{id}]{info}");
    //             } else if let Some(status) = build_info.status {
    //                 let progress = if let Some(progress) = build_info.progress {
    //                     progress
    //                 } else { "".to_string() };
    //                 println!("[{id}] Status: {status} {progress}");
    //             } else {
    //                 println!("non handled build_info {:?}", build_info);
    //             }
    //         }
    //         Err(e) => println!("get error {e}")
    //     }
    // }
    // tokio::fs::remove_file("image.tar").await.unwrap();
    
    Ok(())
}

async fn push_image(image_name: &str) {
    let docker = bollard::Docker::connect_with_local_defaults().unwrap();
    let push_options = bollard::image::PushImageOptions::<&str>::default();
    let credentials = bollard::auth::DockerCredentials { // for sakuracr.jp
        username: Some("username".to_string()),
        password: Some("password".to_string()),
        ..Default::default()
    };
    let mut image_push_stream = docker.push_image(image_name, Some(push_options), Some(credentials));
    while let Some(msg) = image_push_stream.next().await {
        println!("Message: {msg:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Read, path::{Path, PathBuf}};

    #[tokio::test]
    async fn test_bollard_examples() {
        use bollard::Docker;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let version = docker.version().await.unwrap();
        // dbg!(version);
        let list_images_options = bollard::image::ListImagesOptions::<String> { all: true, ..Default::default() };
        let images = docker.list_images(Some(list_images_options)).await.unwrap();
        // dbg!(images);
        for image in images.iter() {
            dbg!(&image.repo_tags);
        }
    }

    #[tokio::test]
    async fn test_build_image() {
        let examples_dir = Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("examples");
        let proj_dir = examples_dir.join("gromacs");
        assert!(proj_dir.exists());
        let image_name = "example_image:test";
        let base_image = "nvcr.io/hpc/gromacs:2023.2";
        build_image(&proj_dir, image_name, base_image).await.unwrap();
    }
}
