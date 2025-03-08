use std::fs::File;
use std::path::Path;

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;



/// download the file from url to file with filepath
pub async fn download(url: &str, filepath: &Path) -> anyhow::Result<()> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let mut file = std::fs::File::create(filepath)?;
    std::io::copy(&mut bytes.as_ref(), &mut file)?;
    Ok(())
}

pub async fn download_async(url: &str, filepath: &Path) -> anyhow::Result<()> {
    let mut file = tokio::fs::File::create(filepath).await?;
    let mut stream = reqwest::get(url).await?.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
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

    #[tokio::test]
    async fn test_download_async() {
        // let filename = "v022.tar.gz";
        let filename = "v022.zip";
        let url = format!("https://github.com/chiral-data/application-examples/archive/refs/tags/{filename}");
        let filepath = home::home_dir().unwrap().join("Downloads").join(filename);
        download_async(&url, &filepath).await.unwrap();
    }
}




