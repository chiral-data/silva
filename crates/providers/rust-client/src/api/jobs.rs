use tonic::transport::Channel;
use tonic::{Request, metadata::MetadataValue};
use std::str::FromStr;
use serde_json::json;
use crate::api::client::chiral::chiral_client::ChiralClient;
use crate::api::client::chiral::RequestUserCommunicate;


pub mod chiral {
    tonic::include_proto!("chiral"); 
}


pub async fn submit_test_job(client: &mut ChiralClient<Channel>, email: &str, token_auth: &str, job_type_name: &str, index: u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let end_point = "SubmitTestJob";
    
    let payload = json!({
        end_point: [job_type_name, index]
    });
    
    let serialized = serde_json::to_string(&payload)?;

    let req_msg = RequestUserCommunicate {
        serialized_request: serialized,
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

pub async fn get_jobs(client: &mut ChiralClient<Channel>, email: &str, token_auth: &str, offset: u32, count_per_page: u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {    let end_point = "GetJobs";
    let _end_point = "GetJobs";
    let serialized = format!(
        "{{\"{}\": [{}, {}]}}",
        end_point, offset, count_per_page
    );

    let req_msg = RequestUserCommunicate {
        serialized_request: serialized.clone(),
    };

    println!("Sending payload: {}", serialized); 

    let mut request = Request::new(req_msg);
    request.metadata_mut().insert("user_id", MetadataValue::from_str(email)?);
    request.metadata_mut().insert("auth_token", MetadataValue::from_str(token_auth)?);

    let response = client.user_communicate(request).await?.into_inner();
    println!("Reply JSON: {}", response.serialized_reply);


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

pub async fn get_job(client: &mut ChiralClient<Channel>, email: &str, token_auth: &str,job_id: &str)->  Result<serde_json::Value, Box<dyn std::error::Error>>{
    let end_point = "GetJob";
    let serialized = format!(
    "{{\"{}\": \"{}\"}}",
    end_point, job_id
    );

    let req_msg = RequestUserCommunicate{
        serialized_request: serialized.clone(),
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
pub async fn submit_job(client: &mut ChiralClient<Channel>, email: &str, token_auth: &str, command_string: &str, project_name: &str, input_files: &[&str], output_files: &[&str]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let end_point = "SubmitJob";

    let input_files_json = serde_json::to_string(&input_files)?;
    let output_files_json = serde_json::to_string(&output_files)?;

    let serialized = format!(
        "{{\"{}\": [\"{}\", \"{}\", {}, {}]}}",
        end_point, command_string, project_name, input_files_json, output_files_json
    );

    let req_msg = RequestUserCommunicate {
        serialized_request: serialized.clone(),
    };

    let mut request = Request::new(req_msg);
    request.metadata_mut().insert("user_id", MetadataValue::from_str(email)?);
    request.metadata_mut().insert("auth_token", MetadataValue::from_str(token_auth)?);

    let response = client.user_communicate(request).await?.into_inner();
    println!("Reply JSON: {}", response.serialized_reply);

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
    use rand::seq::SliceRandom;
    use rand::Rng;

     #[tokio::test]
    async fn test_submit_job(){
        dotenvy::from_filename(".env").ok();
        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL environment variable not set");
        let token_auth = std::env::var("TEST_TOKEN_AUTH").expect("TEST_TOKEN_AUTH environment variable not set");
        let mut client = create_client(&url).await.expect("Failed to create API client. Ensure CHIRAL_STAGING_API_URL is valid and the client can be created.");
        let project_name: &str = "qCEnc6x";
        let input_files = vec!["run.sh", "1aki.pdb"];
        let output_files = vec!["1AKI_processed.gro", "topol.top", "posre.itp"];

        let job_id = submit_job(&mut client, &email, &token_auth, "sh run.sh", project_name, &input_files, &output_files).await.expect("submit_job failed");

        // Assert that the returned value is a non-empty string
        assert!(job_id.is_string(), "Expected job_id to be a string, got: {}", job_id);
        let job_id_str = job_id.as_str().unwrap();
        assert!(!job_id_str.trim().is_empty(), "Job ID should not be empty");
    }

    #[tokio::test]
    async fn test_submit_test_job() {
        dotenvy::from_filename(".env").ok();
        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL environment variable not set");
        let token_auth = std::env::var("TEST_TOKEN_AUTH").expect("TEST_TOKEN_AUTH environment variable not set");

        let mut client = create_client(&url).await.expect("Failed to create API client.");

        let job_types = ["sleep_5_secs", "gromacs_bench_mem"];
        let job_type_name = job_types.choose(&mut rand::thread_rng()).unwrap();
        let index: u32 = rand::thread_rng().gen_range(0..10000);

        let response_json = submit_test_job(&mut client, &email, &token_auth, job_type_name, index)
            .await
            .expect("Submit Test Job Failure");

        println!("SubmitTestJob response:\n{}", response_json);

        assert!(response_json.is_string(), "Expected job ID as string but got: {}", response_json);
        let job_id = response_json.as_str().unwrap();
        assert!(!job_id.trim().is_empty(), "Job ID should not be empty");
    }


    #[tokio::test]
    async fn test_get_job() {
        dotenvy::from_filename(".env").ok();
        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL environment variable not set");
        let token_auth = std::env::var("TEST_TOKEN_AUTH").expect("TEST_TOKEN_AUTH environment variable not set");

        let mut client = create_client(&url).await.expect("Failed to create API client.");
        let project_name: &str = "qCEnc6q";
        let input_files = vec!["run.sh", "1aki.pdb"];
        let output_files = vec!["1AKI_processed.gro", "topol.top", "posre.itp"];

        let job_id_value = submit_job(&mut client, &email, &token_auth, "sh run.sh", project_name, &input_files, &output_files)
            .await
            .expect("SubmitJob failed");

        let job_id_str = job_id_value.as_str().expect("Expected job ID to be a string");

        let get_job_result = get_job(&mut client, &email, &token_auth, job_id_str)
            .await
            .expect("Job result Failed");

        println!("GetJob result: {}", get_job_result);
    }

    
    #[tokio::test]
    async fn test_get_jobs() {
        dotenvy::from_filename(".env").ok();
        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL environment variable not set");
        let token_auth = std::env::var("TEST_TOKEN_AUTH").expect("TEST_TOKEN_AUTH environment variable not set");

        let mut client = create_client(&url).await.expect("Failed to create API client.");

        let json_value = get_jobs(&mut client, &email, &token_auth, 0, 10)
            .await
            .expect("Get Jobs failed");

        let jobs = json_value.as_array().expect("Expected JSON array of jobs");

        assert!(jobs.len() <= 10, "More than 10 jobs returned");

        for job in jobs {
            assert!(job.get("id").is_some(), "Job missing 'id'");
            assert!(job.get("status").is_some(), "Job missing 'status'");
            assert!(job.get("project_name").is_some(), "Job missing 'project_name'");
        }
    }

}