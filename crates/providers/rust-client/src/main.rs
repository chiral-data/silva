mod ftp;
use ftp::FtpClient;
use dotenvy::dotenv;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    /*
    dotenv()?; 

    let addr = env::var("FTP_HOST")?;
    let port: u16 = env::var("FTP_PORT")?.parse()?; 
    let email = env::var("TEST_EMAIL")?;
    let token = env::var("TEST_TOKEN_AUTH")?;
    let user_id = env::var("TEST_ID")?;
    let mut client = FtpClient::new(&addr, port, &email, &token, &user_id);
    */

    let mut client = FtpClient::new(
        "127.0.0.1",     // FTP server address
        21,              // FTP port
        "ftpuser",       // FTP username
        "your_password", // FTP password
        "Aariv_Walia",              // No subfolder (root of ftpuser's home)
    );

    client.connect()?;
    client.download_file("test.txt", "downloaded_test.txt")?;
    client.disconnect();

    Ok(())
}
