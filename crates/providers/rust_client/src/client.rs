use chiral_client;
use std::env;
use anyhow::Result;
use std::error::Error;

#[derive(Debug,Clone)]
pub struct RustClient {
    pub url: String,
    pub user_email: String,
    pub user_id: String,
    pub token_auth: String,
    pub token_api: String,
    pub ftp_addr: String,
    pub ftp_port: u16,
    
}

impl RustClient {
    pub fn new(
        url: String,
        user_email: String,
        user_id: String,
        token_auth: String,
        token_api: String,
        ftp_addr: String,
        ftp_port: u16,
    ) -> Self {
        
        Self {
            url,
            user_email,
            user_id,
            token_auth,
            token_api,
            ftp_addr,
            ftp_port,
        }
    }

    // Constructor that creates instance from environment variables


    pub async fn from_env() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let url = env::var("URL")?;
        let user_email = env::var("USER_EMAIL")?;
        let user_id = env::var("USER_ID")?;
        let token_auth = env::var("TOKEN_AUTH")?;
        let token_api = env::var("TOKEN_API")?;
        let ftp_addr = env::var("FTP_ADDR")?;
        let ftp_port = env::var("FTP_PORT")?.parse::<u16>()?;

        Ok(Self::new(
            url,
            user_email,
            user_id,
            token_auth,
            token_api,
            ftp_addr,
            ftp_port,
        ))
    }



    pub async fn get_credits(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::get_credit_points(&mut client, &self.user_email, &self.token_auth).await
    }

    pub async fn submit_job(&mut self, command_string: &str, project_name: &str, input_files: &[&str], output_files: &[&str]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;        
        chiral_client::submit_job(
            &mut client,
            &self.user_email, 
            &self.token_auth, 
            command_string, 
            project_name, 
            input_files, 
            output_files
        ).await
    }

    pub async fn submit_test_job(&mut self,job_type_name:&str,index:u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::submit_test_job(&mut client, &self.user_email, &self.token_auth,job_type_name,index).await
    }

    pub async fn get_job(&mut self, job_id: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::get_job(&mut client, &self.user_email, &self.token_auth, job_id).await
    }

    pub async fn get_jobs(&mut self, offset: u32, count_per_page: u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;

        chiral_client::get_jobs(&mut client, &self.user_email, &self.token_auth, offset,count_per_page).await
    }

    pub async fn list_projects(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;

        chiral_client::list_of_projects(&mut client, &self.user_email, &self.token_auth).await
    }

    pub async fn list_example_projects(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::list_of_example_projects(&mut client,&self.user_email,&self.token_auth).await
    }

    pub async fn get_project_files(&mut self,project_name: &str,file_name:&str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::get_project_files(&mut client, &self.user_email, &self.token_auth,project_name,file_name).await
    }

    pub async fn list_project_files(&mut self, project_id: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::list_of_project_files(&mut client, &self.user_email, &self.token_auth, project_id).await
    }

    pub async fn import_example_project(&mut self, project_name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::import_example_project(&mut client, &self.user_email, &self.token_auth, project_name).await
    }

    pub async fn get_api_token(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::get_token_api(&mut client, &self.user_email, &self.token_auth).await
    }

    pub async fn refresh_api_token(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let mut client = chiral_client::create_client(&self.url).await?;
        chiral_client::refresh_token_api(&mut client,&self.user_email ,&self.token_api).await
    }


    // Test the connection
    pub async fn test_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.get_credits().await?;
        println!("Connection test successful!");
        Ok(())
    }
}