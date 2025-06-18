use tonic::transport::Channel;
use tonic::{Request, metadata::MetadataValue};
use std::str::FromStr;
use crate::api::client::chiral::chiral_client::ChiralClient;
use crate::api::client::chiral::RequestUserCommunicate;


pub mod chiral {
    tonic::include_proto!("chiral"); 
}

pub async fn get_token_api(client: &mut ChiralClient<Channel>, email: &str, token_auth: &str)->  Result<serde_json::Value, Box<dyn std::error::Error>>{
        let end_point = "GetTokenAPI";
        let serialized = format!(
            "{{\"{}\": null}}",
            end_point
        );

        let req_msg = RequestUserCommunicate{
            serialized_request : serialized.clone(),
        }; 
        let mut request = Request::new(req_msg);

        request.metadata_mut().insert("user_id", MetadataValue::from_str(email)?);
        request.metadata_mut().insert("auth_token", MetadataValue::from_str(token_auth)?);

        let response = client.user_communicate(request).await?.into_inner();
        if !response.serialized_reply.trim().is_empty() {
            let parsed: serde_json::Value = serde_json::from_str(&response.serialized_reply)?;
            if let Some(result) = parsed.get(end_point) {
                return Ok(result.clone());
            } else {
                return Err("Missing endpoint data in server response".into());
            }
        }

        if !response.error.trim().is_empty() {
            return Err(format!("Server error: {}", response.error).into());
        }

        Err("Unexpected empty response from server".into())
}

pub async fn refresh_token_api(client: &mut ChiralClient<Channel>, email: &str, token_auth: &str)->  Result<serde_json::Value, Box<dyn std::error::Error>>{
        let end_point = "RefreshTokenAPI";
        let serialized = format!(
            "{{\"{}\": null}}",
            end_point
        );

        let req_msg = RequestUserCommunicate{
            serialized_request : serialized.clone(),
        }; 
        let mut request = Request::new(req_msg);

        request.metadata_mut().insert("user_id", MetadataValue::from_str(email)?);
        request.metadata_mut().insert("auth_token", MetadataValue::from_str(token_auth)?);

        let response = client.user_communicate(request).await?.into_inner();
        if !response.serialized_reply.trim().is_empty() {
            let parsed: serde_json::Value = serde_json::from_str(&response.serialized_reply)?;
            if let Some(result) = parsed.get(end_point) {
                return Ok(result.clone());
            } else {
                return Err("Missing endpoint data in server response".into());
            }
        }

        if !response.error.trim().is_empty() {
            return Err(format!("Server error: {}", response.error).into());
        }

        Err("Unexpected empty response from server".into())
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::api::create_client;
    use dotenvy;

    #[tokio::test]
    async fn test_get_token_api(){
        dotenvy::from_filename(".env").ok();
        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL environment variable not set");
        let token_auth = std::env::var("TEST_TOKEN_AUTH").expect("TEST_TOKEN_AUTH environment variable not set");

        let mut client = create_client(&url).await.expect("Failed to create API client.");
        let token_api = get_token_api(&mut client, &email, &token_auth).await.expect("Getting Token Failed");
        assert!(!token_api.is_null(), "Returned token API is null");
    }

    #[tokio::test]
    async fn test_refresh_token_api(){
        dotenvy::from_filename(".env").ok();
        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL environment variable not set");
        let token_auth = std::env::var("TEST_TOKEN_AUTH").expect("TEST_TOKEN_AUTH environment variable not set");

        let mut client = create_client(&url).await.expect("Failed to create API client.");
        let refreshed_token = refresh_token_api(&mut client, &email, &token_auth).await.expect("Failed to refresh Token");
        assert!(!refreshed_token.is_null(),"Refreshed token API is null");
    }
}