mod ftp;
use ftp::FtpClient;
use dotenvy::dotenv;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?; 

    let addr = env::var("FTP_HOST")?;
    let port: u16 = env::var("FTP_PORT")?.parse()?; 
    let email = env::var("TEST_EMAIL")?;
    let token = env::var("TEST_TOKEN_AUTH")?;
    let user_id = env::var("TEST_ID")?;

    let mut client = FtpClient::new(&addr, port, &email, &token, &user_id);

    client.connect()?;
    client.disconnect();

    Ok(())
}
