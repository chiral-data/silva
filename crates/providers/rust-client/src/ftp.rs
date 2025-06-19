use ftp::FtpStream;
use std::fs::File;
use std::io::{Write,Read};


#[derive(Debug, PartialEq)]
enum PathType {
    NotExist,
    File,
    Directory,
}
#[derive(Debug)]
pub enum FtpError {
    ConnectionError(std::io::Error),
    LoginError,
    DirectoryError(String),
    FileError(String),
}

#[derive(Debug)]
pub struct FtpClient {
    ftp_addr: String,
    ftp_port: u16,
    user_email: String,
    token_api: String,
    user_id: String,
    ftp: Option<FtpStream>,
    root_dir: Option<String>,
}

impl FtpClient {
    pub fn new(addr: &str, port: u16, email: &str, token: &str, user_id: &str) -> Self {
        FtpClient {
            ftp_addr: addr.to_string(),
            ftp_port: port,
            user_email: email.to_string(),
            token_api: token.to_string(),
            user_id: user_id.to_string(),
            ftp: None,
            root_dir: None,
        }
    }

    pub fn connect(&mut self) -> Result<(), ftp::FtpError> {
        let address = format!("{}:{}", self.ftp_addr, self.ftp_port);
        let mut ftp_stream = FtpStream::connect(address)?;
        ftp_stream.login(&self.user_email, &self.token_api)?;
        
        // Try to change into the user's subdirectory
        ftp_stream.cwd(&self.user_id)?;
        
        // Confirm we're in the correct directory
        let current_dir = ftp_stream.pwd()?;
        println!("Connected and changed to directory: {}", current_dir);

        self.root_dir = Some(current_dir);
        self.ftp = Some(ftp_stream);
        Ok(())
    }


    pub fn disconnect(&mut self) {
        if let Some(mut ftp) = self.ftp.take() {
            let _ = ftp.quit(); // ignore error
        }
        self.root_dir = None;
    }

    pub fn is_connected(&self) -> bool {
    self.ftp.is_some()
    }

    pub fn download_file(&mut self, remote_path: &str, local_path: &str) -> Result<(), ftp::FtpError> {
        let ftp_stream = match &mut self.ftp {
            Some(ftp) => ftp,
            None => {
                return Err(ftp::FtpError::ConnectionError(
                    std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to FTP server"),
                ))
            }
        };

        let data = ftp_stream.simple_retr(remote_path)?;

        let mut file = File::create(local_path).map_err(|e| {
            ftp::FtpError::ConnectionError(e)
        })?;

        file.write_all(&data.into_inner()).map_err(|e| {
            ftp::FtpError::ConnectionError(e)
        })?;

        Ok(())
    }

    pub fn upload_file(&mut self, local_path: &str, remote_path: &str) -> Result<(), ftp::FtpError> {
        let ftp_stream = match &mut self.ftp {
            Some(ftp) => ftp,
            None => {
                return Err(ftp::FtpError::ConnectionError(
                    std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to FTP server"),
                ))
            }
        };

        let mut file = File::open(local_path).map_err(ftp::FtpError::ConnectionError)?;
        ftp_stream.put(remote_path, &mut file)?;

        Ok(())
    }


    pub fn make_directory(&mut self, dir_name: &str) -> Result<(), ftp::FtpError> {
        let ftp_stream = match &mut self.ftp {
            Some(ftp) => ftp,
            None => {
                return Err(ftp::FtpError::ConnectionError(
                    std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to FTP server"),
                ))
            }
        };

        ftp_stream.mkdir(dir_name)?;
        println!("Created directory: {}", dir_name);

        Ok(())
    }


    pub fn change_directory(&mut self, dir: &str) -> Result<(), ftp::FtpError> {
        let ftp_stream = match &mut self.ftp {
            Some(ftp) => ftp,
            None => {
                return Err(ftp::FtpError::ConnectionError(
                    std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to FTP server"),
                ))
            }
        };  
        ftp_stream.cwd(dir)?;
        let current_dir = ftp_stream.pwd()?;
        self.root_dir = Some(current_dir.clone());

        println!("Changed directory to: {}", current_dir);

        Ok(())
    }

    pub fn remove_directory_recursive(&mut self, dir_path: &str) -> Result<(), ftp::FtpError> {
        let mut ftp_stream = self.ftp.take().ok_or_else(|| {
            ftp::FtpError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected to FTP server",
            ))
        })?;

        ftp_stream.cwd(dir_path)?;

        let entries = ftp_stream.nlst(None)?;

        for entry in entries {
            if ftp_stream.cwd(&entry).is_ok() {
                ftp_stream.cdup()?; 
                let subdir_path = format!("{}/{}", dir_path, entry);
                
                self.ftp = Some(ftp_stream);
                self.remove_directory_recursive(&subdir_path)?;
                ftp_stream = self.ftp.take().unwrap(); 
            } else {
                let file_path = format!("{}/{}", dir_path, entry);
                ftp_stream.rm(&file_path)?;
            }
        }

        ftp_stream.cdup()?;
        ftp_stream.rmdir(dir_path)?;

        println!("Removed directory recursively: {}", dir_path);

        self.ftp = Some(ftp_stream);

        Ok(())
    }

}

impl Drop for FtpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
