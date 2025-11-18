use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone)]
pub struct HealthCheckItem {
    pub name: String,
    pub status: HealthStatus,
    pub details: String,
}

#[derive(Clone, PartialEq)]
pub enum HealthStatus {
    Pass,
    Fail,
    Warning,
    Checking,
}

#[derive(Default)]
pub struct State {
    pub health_checks: Vec<HealthCheckItem>,
}

impl State {
    pub fn handle_input(&mut self, key: KeyEvent) {
        if let KeyCode::Char('r') = key.code {
            self.run_health_checks();
        }
    }

    pub fn run_health_checks(&mut self) {
        self.health_checks.clear();

        // Environment variables check
        self.check_environment_variables();

        // Software installation check
        self.check_software_installations();
    }

    pub fn check_environment_variables(&mut self) {
        let env_vars = vec!["SILVA_WORKFLOW_HOME", "SHELL"];

        for var in env_vars {
            let status = if std::env::var(var).is_ok() {
                HealthStatus::Pass
            } else {
                HealthStatus::Fail
            };

            let details = std::env::var(var).unwrap_or_else(|_| "Not set".to_string());

            self.health_checks.push(HealthCheckItem {
                name: format!("Environmental vairables: {var}"),
                status,
                details: if details.len() > 50 {
                    format!("{}...", &details[..47])
                } else {
                    details
                },
            });
        }
    }

    pub fn check_software_installations(&mut self) {
        let software = vec!["docker"];

        // Use "where" on Windows, "which" on Unix-like systems
        let command = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };

        for sw in software {
            let output = std::process::Command::new(command).arg(sw).output();

            let (status, details) = match output {
                Ok(result) if result.status.success() => {
                    let path = String::from_utf8_lossy(&result.stdout).trim().to_string();
                    (HealthStatus::Pass, path)
                }
                _ => (HealthStatus::Fail, "Not found".to_string()),
            };

            self.health_checks.push(HealthCheckItem {
                name: format!("Software: {sw}"),
                status,
                details,
            });
        }
    }
}
