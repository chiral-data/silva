use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::constants;

pub fn get_data_dir() -> PathBuf {
    app_dirs2::app_root(app_dirs2::AppDataType::UserData, &constants::APP_INFO).unwrap()
}

/// download the file from url to file with filepath
pub async fn download(url: &str, filepath: &Path) -> anyhow::Result<()> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let mut file = std::fs::File::create(filepath)?;
    std::io::copy(&mut bytes.as_ref(), &mut file)?;
    Ok(())
}

pub fn unzip_tar_gz(filepath: &Path, to_folder: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(to_folder)?;

    let file = File::open(filepath)?;
    let gz_decoder = flate2::read::GzDecoder::new(file);
    let mut ar = tar::Archive::new(gz_decoder);

    for entry in ar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.is_dir() {
            std::fs::create_dir_all(to_folder.join(path))?
        } else {
            let mut file = File::create(to_folder.join(path))?;
            std::io::copy(&mut entry, &mut file)?;
        }
    }

    Ok(())
}

pub fn get_child_dirs<P: AsRef<Path>>(dir: P) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(dir).unwrap()
        .filter_map(|entry| match entry {
            Ok(e) => {
                if e.path().is_dir() {
                    e.path().to_str().map(PathBuf::from)
                } else { None }
            }
            Err(_) => None
        })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dok_artifact() {
        let client = sacloud_rs::Client::default().dok();
        let id = "ed1fd80e-aa00-4666-8cd1-04b624373d92";
        let task: sacloud_rs::api::dok::Task = client.clone()
            .tasks().task_id(id).dok_end().get()
            .await.unwrap();
        let artifact_url: sacloud_rs::api::dok::ArtifactUrl = client
            .artifacts().artifact_id(&task.artifact.unwrap().id).download().dok_end()
            .get().await.unwrap();
        let filepath = home::home_dir().unwrap().join("Downloads").join("1.tar.gz");
        download(&artifact_url.url, &filepath).await.unwrap();
        let to_folder = home::home_dir().unwrap().join("Downloads").join("1");
        unzip_tar_gz(&filepath, &to_folder).unwrap();
    }
}




