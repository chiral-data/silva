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
