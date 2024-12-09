use crate::Client;

pub async fn get_registries(client: Client) -> anyhow::Result<super::RegistryList> {
    let registry_list: super::RegistryList = client.clone().registries().dok_end().get()
        .await?;
    Ok(registry_list)
}

pub async fn create_task(client: Client, image_name: &str, registry: &super::Registry, plan: super::params::Plan) -> anyhow::Result<super::TaskCreated> {
    let container = super::params::Container::default()
        .image(image_name.to_string())
        .registry(Some(registry.id.to_string()))
        .command(vec![])
        .entrypoint(vec![])
        .plan(plan);
    let post_tasks = super::params::PostTasks::default()
        .name("some_task".to_string())
        .containers(vec![container])
        .tags(vec![]);
    let task_created: super::TaskCreated = client.tasks().dok_end()
        .set_params(&post_tasks)?
        .post().await?;

    Ok(task_created)
}
