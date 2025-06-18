use tonic::transport::Channel;

pub mod chiral {
    tonic::include_proto!("chiral"); 
}

use chiral::chiral_client::ChiralClient;

pub async fn create_client(url: &str) -> Result<ChiralClient<Channel>, Box<dyn std::error::Error>> {
    Ok(ChiralClient::connect(url.to_string()).await?)
}

#[cfg(test)]
mod tests{
    use super::create_client;
    #[tokio::test]
    async fn test_create_client(){
        dotenvy::from_filename(".env").ok();

        let url = std::env::var("CHIRAL_STAGING_API_URL").expect("CHIRAL_STAGING_API_URL environment variable not set");
        let result = create_client(&url).await;
        assert!(result.is_ok(), "Client creation failed: {:?}", result.err());
    }
}