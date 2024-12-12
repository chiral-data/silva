use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Hash)]
pub enum App {
    #[default]
    Unknown,
    Gromacs,
    Psi4,
    MyPresto,
}

impl From<&str> for App {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "gromacs" => App::Gromacs,
            "psi4" => App::Psi4,
            "mypresto" => App::MyPresto,
            _ => App::Unknown
        }
    }
}

impl App {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Gromacs => "Gromacs",
            Self::Psi4 => "Psi4",
            Self::MyPresto => "myPresto",
            Self::Unknown => "Unknown"
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Manager {
    pub apps: Vec<App>,
}

impl Manager {
    pub fn new() -> Self {
        // TODO: temporarily hard coding
        let apps = vec!["gromacs"].into_iter()
            .map(App::from)
            .collect();
        Self { apps }
    }
}

