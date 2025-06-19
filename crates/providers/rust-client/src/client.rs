use crate::api::credits;
use crate::api::token;
use crate::api::jobs;
use crate::api::projects;
use crate::api::client::chiral::chiral_client::ChiralClient;
use crate::ftp::{FtpClient, FtpError};
use tonic::transport::Channel;
use serde_json::Value;



#[derive(Debug)]
pub struct Client {
    pub email: String,
    pub token: String,
    pub endpoint: String,
    pub inner: ChiralClient<Channel>,
    pub ftp_client: Option<FtpClient>,
}

impl Client {
    pub fn new(email: String, token: String, endpoint: String, inner: ChiralClient<Channel>) -> Self {
        Self { 
            email, 
            token, 
            endpoint, 
            inner,
            ftp_client: None,
        }
    }

    pub async fn connect_with_auth(endpoint: &str, email: String, token: String) -> Result<Self, Box<dyn std::error::Error>> {
        let inner = ChiralClient::connect(endpoint.to_string()).await?;
        Ok(Self {
            email,
            token,
            endpoint: endpoint.to_string(),
            inner,
            ftp_client: None,
        })
    }

    pub async fn get_credit_points(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        credits::get_credit_points(&mut self.inner, &self.email, &self.token).await
    }

    pub async fn submit_job(&mut self,command_string: &str,project_name: &str,input_files: &[&str],output_files: &[&str],) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        jobs::submit_job(&mut self.inner,&self.email,&self.token,command_string,project_name,input_files,output_files,).await
    }

    pub async fn submit_test_job(&mut self,job_type_name: &str,index: u32,) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        jobs::submit_test_job(&mut self.inner, &self.email, &self.token, job_type_name, index).await
    }

    pub async fn get_jobs(&mut self,offset: u32,count_per_page: u32,) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        jobs::get_jobs(&mut self.inner, &self.email, &self.token, offset, count_per_page).await
    }

    pub async fn get_job(&mut self, job_id: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        jobs::get_job(&mut self.inner, &self.email, &self.token, job_id).await
    }

    // Project API methods
    pub async fn list_of_projects(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        projects::list_of_projects(&mut self.inner, &self.email, &self.token).await
    }

    pub async fn list_of_example_projects(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        projects::list_of_example_projects(&mut self.inner, &self.email, &self.token).await
    }

    pub async fn list_of_project_files(&mut self, project_name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        projects::list_of_project_files(&mut self.inner, &self.email, &self.token, project_name).await
    }

    pub async fn import_example_project(&mut self, project_name: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        projects::import_example_project(&mut self.inner, &self.email, &self.token, project_name).await
    }

    pub async fn get_project_file(&mut self,project_name: &str,file_name: &str,) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        projects::get_project_files(&mut self.inner, &self.email, &self.token, project_name, file_name).await
    }

    // Token API methods
    pub async fn get_token_api(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        token::get_token_api(&mut self.inner, &self.email, &self.token).await
    }

    pub async fn refresh_token_api(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        token::refresh_token_api(&mut self.inner, &self.email, &self.token).await
    }

    // FTP methods
    pub fn initialize_ftp(&mut self, ftp_addr: &str, ftp_port: u16, user_id: &str) {
        let ftp_client = FtpClient::new(ftp_addr, ftp_port, &self.email, &self.token, user_id);
        self.ftp_client = Some(ftp_client);
    }

    pub fn connect_ftp(&mut self) -> Result<(), ftp::FtpError> {
        match &mut self.ftp_client {
            Some(ftp) => ftp.connect(),
            None => Err(ftp::FtpError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "FTP client not initialized. Call initialize_ftp() first.")
            ))
        }
    }

    pub fn disconnect_ftp(&mut self) {
        if let Some(ftp) = &mut self.ftp_client {
            ftp.disconnect();
        }
    }

    pub fn is_ftp_connected(&self) -> bool {
        self.ftp_client.as_ref().map_or(false, |ftp| ftp.is_connected())
    }

    pub fn download_file(&mut self, remote_path: &str, local_path: &str) -> Result<(), ftp::FtpError> {
        match &mut self.ftp_client {
            Some(ftp) => ftp.download_file(remote_path, local_path),
            None => Err(ftp::FtpError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "FTP client not initialized. Call initialize_ftp() first.")
            ))
        }
    }

    pub fn upload_file(&mut self, local_path: &str, remote_path: &str) -> Result<(), ftp::FtpError> {
        match &mut self.ftp_client {
            Some(ftp) => ftp.upload_file(local_path, remote_path),
            None => Err(ftp::FtpError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "FTP client not initialized. Call initialize_ftp() first.")
            ))
        }
    }

    pub fn make_directory(&mut self, dir_name: &str) -> Result<(), ftp::FtpError> {
        match &mut self.ftp_client {
            Some(ftp) => ftp.make_directory(dir_name),
            None => Err(ftp::FtpError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "FTP client not initialized. Call initialize_ftp() first.")
            ))
        }
    }

    pub fn change_directory(&mut self, dir: &str) -> Result<(), ftp::FtpError> {
        match &mut self.ftp_client {
            Some(ftp) => ftp.change_directory(dir),
            None => Err(ftp::FtpError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "FTP client not initialized. Call initialize_ftp() first.")
            ))
        }
    }

    pub fn remove_directory_recursive(&mut self, dir_path: &str) -> Result<(), ftp::FtpError> {
        match &mut self.ftp_client {
            Some(ftp) => ftp.remove_directory_recursive(dir_path),
            None => Err(ftp::FtpError::ConnectionError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "FTP client not initialized. Call initialize_ftp() first.")
            ))
        }
    }

    pub fn get_ftp_client(&mut self) -> Option<&mut FtpClient> {
        self.ftp_client.as_mut()
    }
}