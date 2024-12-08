//! Sakura Internet API: Koukaryouku DOK GPU Container Service 
//! https://manual.sakura.ad.jp/koukaryoku-dok-api/spec.html
//!
//! [attention!]
//! Access permission for other services from the API Key page
//!     Koukaryouku DOK should be clicked
//!
//! Auth
//!     - [] GET        /auth/
//!     - [] POST       /auth/agree/
//! Registry
//!     - [] GET        /registries/
//!     - [] POST       /registries/
//!     - [] GET        /registries/{registryID}/
//!     - [] DELETE     /registries/{registryID}/
//!     - [] PUT        /registries/{registryID}/
//! Task
//!     - [x] GET       /tasks/
//!     - [] POST       /tasks/
//!     - [x] GET       /tasks/{taskId}/
//!     - [] DELETE     /tasks/{taskId}/
//!     - [] POST       /tasks/{taskId}/cancel/
//!     - [] GET        /tasks/{taskId}/download/{target}/
//! Artifacts
//!     - [] GET        /artifacts/
//!     - [] GET        /artifacts/{artifactId}
//!     - [x] GET       /artifacts/{artifactId}/download/

use serde::{Deserialize, Serialize};

create_struct!(Artifact, "lowercase",
    id: String,
    filename: String
);

create_struct!(Meta, "lowercase",
    page: usize,
    page_size: usize,
    count: usize
);

create_struct!(Registry, "lowercase",
);

create_struct!(RegistryList, "lowercase",
);

create_struct!(Task, "lowercase",
    id: String,
    status: String,
    artifact: Artifact
);

create_struct!(TaskList, "lowercase",
    meta: Meta,
    results: Vec<Task>
);


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[tokio::test]
    async fn test_get_registries() {
        let client = Client::default().dok();
        client.registries().dok_end().get()
    }

    #[tokio::test]
    async fn test_get_tasks() {
        let client = Client::default().dok();
        let task_list: TaskList = client
            .tasks().dok_end().get()
            .await.unwrap();
        dbg!(task_list);
    }

    #[tokio::test]
    async fn test_post_tasks() {
        todo!()
    }

    #[tokio::test]
    async fn test_get_task() {
        let client = Client::default().dok();
        let id = "36400d29-3d9a-4b4a-a5b9-2037f3efa257";
        let task: Task = client
            .tasks().task_id(id).dok_end().get()
            .await.unwrap();
        dbg!(task);
    }

    #[tokio::test]
    async fn test_get_artifact_download_url() {
        let client = Client::default().dok();
        let id = "de505312-5b49-4c03-8f0b-6bc317aac848";
        let task: Task = client.clone()
            .tasks().task_id(id).dok_end().get()
            .await.unwrap();
        client.artifacts().artifact_id(&task.artifact.id).download().dok_end().get_raw().await;
    }
}

pub mod params;
