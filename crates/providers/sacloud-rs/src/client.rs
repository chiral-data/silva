use reqwest::Response;
use serde::{de::DeserializeOwned, Serialize};
use std::env;

use crate::Zone;

const BASE_URL: &str = "https://secure.sakura.ad.jp/cloud/zone";

#[derive(Clone, Debug)]
pub struct Client {
    key_1: String,
    key_2: Option<String>,
    zone: Zone,
    service: String, // "cloud" for Normal API, "managed-container" for DOK API
    api_version: String, // 1.1 for Normal API, 1.0 for DOK API
    url: String,
    body: String,
}

impl Default for Client {
    fn default() -> Self {
        let key_1 = env::var("SAKURA_KEY1").unwrap();
        let key_2 = env::var("SAKURA_KEY2").ok();
        Self::new(key_1, key_2)
    }
}

impl Client {
    pub fn new(key_1: String, key_2: Option<String>) -> Self {
        let zone = Zone::Tokyo1;
        Self {
            key_1,
            key_2,
            zone,
            service: "cloud".to_string(),
            api_version: "1.1".to_string(),
            url: String::new(),
            body: String::new(),
        }
    }

    pub fn dok(mut self) -> Self {
        self.service = "managed-container".to_string();
        self.api_version = "1.0".to_string();
        self.zone = Zone::Ishikari1;
        self
    }

    pub fn get_zone(&self) -> Zone {
        self.zone
    }

    pub fn set_zone(mut self, zone: Zone) -> Self {
        self.zone = zone;
        self
    }

    pub fn clear(mut self) -> Self {
        self.url.clear();
        self.body.clear();
        self
    }

    pub fn set_params<P: Serialize>(mut self, params: &P) -> anyhow::Result<Self> {
        self.body = serde_json::to_string(params)?;
        Ok(self)
    }

    // for test purpose
    pub async fn get_raw(&self) {
        println!("Request URL: {}", self.full_url());
        let client = reqwest::Client::new();
        let res = client
            .get(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .send()
            .await
            .unwrap();
        if let Ok(text_response) = res.text().await {
            println!("Response Text:\n {} \n", &text_response);
            let v: serde_json::Value = serde_json::from_str(&text_response).unwrap();
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        } else {
            panic!("no text response");
        }
    }

    pub async fn get<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
       let client = reqwest::Client::new();
        let res = client
            .get(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .send()
            .await?;
        let t: T = res.json().await?;
        Ok(t)
    }

    pub async fn get_with_params<T: DeserializeOwned, K: AsRef<str>, V: AsRef<str>>(&self, params: &[(K, V)]) -> anyhow::Result<T> {
        let url = reqwest::Url::parse_with_params(self.full_url().as_str(), params)?;
        let client = reqwest::Client::new();
        let res = client
            .get(url)
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .send()
            .await?;
        let t: T = res.json().await?;
        Ok(t)
    }

    pub async fn post_raw(&self) {
        let client = reqwest::Client::new();
        let res = client
            .post(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .body(self.body.clone())
            .send()
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&res.text().await.unwrap()).unwrap();
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }

    pub async fn post<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let client = reqwest::Client::new();
        let res = client
            .post(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .body(self.body.clone())
            .send()
            .await?;
        let t: T = res.json().await?;
        Ok(t)
    }

    pub async fn delete_raw(&self) {
        let client = reqwest::Client::new();
        let res = client.delete(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .body(self.body.clone())
            .send()
            .await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&res.text().await.unwrap()).unwrap();
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }

    pub async fn delete(&self) -> anyhow::Result<Response> {
        let client = reqwest::Client::new();
        client
            .delete(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .body(self.body.clone())
            .send()
            .await
            .map_err(|e| anyhow::Error::msg(format!("{e}")))
    }

    pub async fn put_raw(&self) {
        let client = reqwest::Client::new();
        let res = client
            .put(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .body(self.body.clone())
            .send()
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&res.text().await.unwrap()).unwrap();
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }

    pub async fn put(&self) -> anyhow::Result<Response> {
        let client = reqwest::Client::new();
        let res = client
            .put(self.full_url())
            .basic_auth(&self.key_1, self.key_2.as_ref())
            .body(self.body.clone())
            .send()
            .await?;
        Ok(res)
    }
}

impl Client {
    pub fn full_url(&self) -> String {
        // Normal API URL
        // https://secure.sakura.ad.jp/cloud/zone/tk1a/api/cloud/1.1/
        // DOK API 
        // https://secure.sakura.ad.jp/cloud/zone/is1a/api/managed-container/1.0/
        format!(
            "{}/{}/api/{}/{}{}",
            BASE_URL, self.zone, self.service, self.api_version, self.url
        )
    }

    fn extend_url(mut self, s: &str) -> Self {
        self.url += s;
        self
    }

    pub fn archive(self) -> Self { self.extend_url("/archive") }
    pub fn archiveid(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) }
    pub fn artifacts(self) -> Self { self.extend_url("/artifacts") }
    pub fn artifact_id(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) } 
    pub fn auth(self) -> Self  { self.extend_url("/auth") }
    pub fn config(self) -> Self { self.extend_url("/config") }
    pub fn disk(self) -> Self { self.extend_url("/disk") }
    pub fn diskid(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) }
    pub fn download(self) -> Self { self.extend_url("/download") }
    pub fn dok_end(self) -> Self { self.extend_url("/") }
    pub fn generate(self) -> Self { self.extend_url("/generate") }
    pub fn interface(self) -> Self { self.extend_url("/interface") }
    pub fn interfaceid(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) }
    pub fn power(self) -> Self { self.extend_url("/power") }
    pub fn product(self) -> Self { self.extend_url("/product") }
    pub fn registries(self) -> Self { self.extend_url("/registries") }
    pub fn server(self) -> Self { self.extend_url("/server") }
    pub fn serverid(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) }
    pub fn shared(self) -> Self { self.extend_url("/shared") }
    pub fn sshkey(self) -> Self { self.extend_url("/sshkey") }
    pub fn sshkeyid(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) }
    pub fn switch(self) -> Self { self.extend_url("/switch") }
    pub fn tasks(self) -> Self { self.extend_url("/tasks") }
    pub fn task_id(self, id: &str) -> Self { self.extend_url(format!("/{id}").as_str()) }
    pub fn to(self) -> Self { self.extend_url("/to") }
}

