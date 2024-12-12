use crate::Client;
use crate::api::dok;

pub async fn get_registries(client: Client) -> anyhow::Result<dok::RegistryList> {
    let registry_list: super::RegistryList = client.dok().registries().dok_end().get()
        .await?;
    Ok(registry_list)
}

pub async fn create_task(client: Client, image_name: &str, registry_id: &str, plan: dok::params::Plan) -> anyhow::Result<dok::TaskCreated> {
    let container = super::params::Container::default()
        .image(image_name.to_string())
        .registry(Some(registry_id.to_string()))
        .command(vec![])
        .entrypoint(vec![])
        .plan(plan);
    let post_tasks = super::params::PostTasks::default()
        .name("some_task".to_string())
        .containers(vec![container])
        .tags(vec![]);
    let task_created: super::TaskCreated = client.dok().tasks().dok_end()
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

pub async fn get_artifact_download_url(client: Client, task: &dok::Task) -> anyhow::Result<dok::ArtifactUrl>{
    let artifact_id = task.artifact.as_ref()
        .map(|af| &af.id)
        .ok_or(anyhow::Error::msg(format!("no artifact for task {}", task.id)))?;
    let artifact_url: dok::ArtifactUrl = client.dok()
        .artifacts().artifact_id(artifact_id).download().dok_end()
        .get().await?;
    Ok(artifact_url)
}
