use std::process::Command;
use std::env;

/// Checks if a specified environment variable is set.
///
/// Returns `Ok(value)` if the variable is set, or `Err(message)` if it's not.
fn check_env_variable(var_name: &str) -> anyhow::Result<String> {
    match env::var(var_name) {
        Ok(_value) => Ok(format!("Environment variable '{}' is set", var_name)),
        Err(_) => Err(anyhow::Error::msg(format!("Environment variable '{}' is NOT set.", var_name))),
    }
}

/// Checks if Docker is installed and executable by running `docker --version`.
///
/// Returns `Ok(version_info)` if Docker is found, or `Err(message)` if it's not.
fn check_docker_installation() -> anyhow::Result<String> {
    let output = Command::new("docker")
        .arg("--version")
        .output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let version_info = String::from_utf8_lossy(&output.stdout);
                Ok(format!("Docker is installed. Version: {}", version_info.trim()))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow::Error::msg(format!("Docker command failed (exit code: {}). Is Docker installed and in PATH? Error: {}", output.status, stderr.trim())))
            }
        },
        Err(e) => Err(anyhow::Error::msg(format!("Failed to execute 'docker' command. Is Docker installed and in PATH? Error: {}", e))),
    }
}


pub fn check_chiral_service() -> anyhow::Result<String> {
    let env_var_1 = "SILVA_CHIRAL_USERNAME";
    let env_var_2 = "SILVA_CHIRAL_API_TOKEN";
    let _ = check_env_variable(env_var_1)?;
    let _ = check_env_variable(env_var_2)?;
    Ok(format!("Environment variables {env_var_1} and {env_var_2} are set. Use Chiral services for computation."))
}

pub fn check_local_computer() -> anyhow::Result<String> {
    check_docker_installation()
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper to ensure environment variables are cleaned up after tests
    struct EnvVarGuard {
        name: String,
        original_value: Option<String>,
    }

    impl EnvVarGuard {
        fn new(name: &str) -> Self {
            let original_value = env::var(name).ok();
            EnvVarGuard {
                name: name.to_string(),
                original_value,
            }
        }

        fn set(&self, value: &str) {
            env::set_var(&self.name, value);
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original_value {
                env::set_var(&self.name, value);
            } else {
                env::remove_var(&self.name);
            }
        }
    }

    #[test]
    fn test_check_env_variable_set() {
        let var_name = "TEST_ENV_VAR_SET";
        let guard = EnvVarGuard::new(var_name);
        guard.set("test_value");
        let result = check_env_variable(var_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), format!("Environment variable '{var_name}' is set"));
    }

    #[test]
    fn test_check_env_variable_not_set() {
        let var_name = "TEST_ENV_VAR_NOT_SET";
        let _guard = EnvVarGuard::new(var_name); // Ensure it's not set
        env::remove_var(var_name);
        let result = check_env_variable(var_name);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Environment variable '{var_name}' is NOT set.")
        );
    }

    #[test]
    fn test_check_docker_installation_success() {
        let result = check_docker_installation();
        if let Ok(msg) = result {
            assert!(msg.contains("Docker is installed. Version:"));
            println!("test_check_docker_installation_success: {}", msg);
        } else {
            eprintln!("Warning: Docker not found or command failed during test_check_docker_installation_success. This test requires Docker to be installed.");
            eprintln!("Error: {:?}", result.unwrap_err());
        }
    }

    #[test]
    fn test_check_chiral_service() {
        let var_name_1 = "SILVA_CHIRAL_USERNAME";
        let var_name_2 = "SILVA_CHIRAL_API_TOKEN";

        // test all set
        {
            let _guard1 = EnvVarGuard::new(var_name_1);
            let _guard2 = EnvVarGuard::new(var_name_2);
            env::set_var(var_name_1, "user");
            env::set_var(var_name_2, "token");
            let result = check_chiral_service();
            assert!(result.is_ok());
            assert_eq!(
                result.unwrap(),
                format!("Environment variables {var_name_1} and {var_name_2} are set. Use Chiral services for computation.")
            );
        }

        // test username not set
        {
            let _guard1 = EnvVarGuard::new(var_name_1);
            let _guard2 = EnvVarGuard::new(var_name_2);
            env::remove_var(var_name_1); 
            env::set_var(var_name_2, "token");
            let result = check_chiral_service();
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                format!("Environment variable '{var_name_1}' is NOT set.")
            );
        }

        // test api_token not set
        {
            let _guard1 = EnvVarGuard::new(var_name_1);
            let _guard2 = EnvVarGuard::new(var_name_2);
            env::set_var(var_name_1, "user");
            env::remove_var(var_name_2); 
            let result = check_chiral_service();
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                format!("Environment variable '{var_name_2}' is NOT set.")
            );
        }
    }

    #[test]
    fn test_check_local_computer_success() {
        let result = check_local_computer();
        if let Ok(msg) = result {
            assert!(msg.contains("Docker is installed. Version:"));
            println!("test_check_local_computer_success: {}", msg);
        } else {
            eprintln!("Warning: Docker not found or command failed during test_check_local_computer_success. This test requires Docker to be installed.");
            eprintln!("Error: {:?}", result.unwrap_err());
        }
    }

    #[test]
    fn test_check_local_computer_failure() {
        let guard = EnvVarGuard::new("PATH");
        guard.set("/another/non/existent/path_for_test");
        let result = check_docker_installation();
        assert!(result.is_err());
        let result = check_local_computer();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to execute 'docker' command.")
                || err_msg.contains("Docker command failed")
                || err_msg.contains("No such file or directory")
                || err_msg.contains("program not found"),
            "Unexpected error message: {}",
            err_msg
        );
    }
}
