use crate::Client;
use crate::api::dok;

pub async fn get_registries(client: Client) -> anyhow::Result<dok::RegistryList> {
    let registry_list: dok::RegistryList = client.dok()
        .registries().dok_end().get()
        .await?;
    Ok(registry_list)
}

pub async fn create_registry(client: Client, hostname: &str, username: &str, password: &str) -> anyhow::Result<dok::Registry> {
    let post_registries = dok::params::PostRegistries::default()
        .hostname(hostname.to_string())
        .username(username.to_string())
        .password(password.to_string());
    let registry: dok::Registry = client.dok()
        .registries().dok_end()
        .set_params(&post_registries)?
        .post().await?;

    Ok(registry)
}

pub async fn create_task(client: Client, container: dok::params::Container) -> anyhow::Result<dok::TaskCreated> {
    let post_tasks = dok::params::PostTasks::default()
        .name("some_task".to_string())
        .containers(vec![container])
        .tags(vec![]);
    let task_created: dok::TaskCreated = client.dok()
        .tasks().dok_end()
        .set_params(&post_tasks)?
        .post().await?;

    Ok(task_created)
}


pub async fn get_task(client: Client, task_id: &str) -> anyhow::Result<dok::Task> {
    let task: dok::Task = client.dok()
        .tasks().task_id(task_id).dok_end().get()
        .await?;
    Ok(task)
}

pub async fn cancel_task(client: Client, task_id: &str) -> anyhow::Result<dok::Task> {
    client.dok().tasks()
        .task_id(task_id).cancel().dok_end()
        .post().await
}

pub async fn get_artifact_download_url(client: Client, task: &dok::Task) -> anyhow::Result<dok::ArtifactUrl>{
    let artifact_id = task.artifact.as_ref()
        .map(|af| &af.id)
        .ok_or(anyhow::Error::msg(format!("no artifact for task {}", task.id)))?;
    let artifact_url: dok::ArtifactUrl = client.dok()
        .artifacts().artifact_id(artifact_id).download().dok_end()
        .get().await?;
    Ok(artifact_url)
}
