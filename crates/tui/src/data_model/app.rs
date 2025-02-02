use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Hash)]
pub enum App {
    #[default]
    Unknown,
    Gromacs,
    OpenAIWhisper,
    Llm,
    MyPresto,
    Psi4,
}

impl From<&str> for App {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "gromacs" => App::Gromacs,
            "psi4" => App::Psi4,
            "mypresto" => App::MyPresto,
            "openai_whisper" => App::OpenAIWhisper,
            "llm" => App::Llm,
            _ => App::Unknown
        }
    }
}

impl App {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::Gromacs => "Gromacs",
            Self::OpenAIWhisper => "OpenAI Whisper",
            Self::Llm => "LLM",
            Self::MyPresto => "myPresto",
            Self::Psi4 => "Psi4",
        }
    }

    pub fn keywords(&self) -> &str {
        match self {
            Self::Unknown => "Unknown",
            Self::Gromacs => "molecular simulation",
            Self::MyPresto => "myPresto",
            Self::OpenAIWhisper => "speech recognition",
            Self::Llm => "large language model",
            Self::Psi4 => "Psi4",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Manager {
    pub apps: Vec<App>,
}

impl Manager {
    pub fn new() -> Self {
        let apps = vec![
            App::Gromacs, App::OpenAIWhisper, App::Llm,
        ];
        Self { apps }
    }

    // pub fn selected(&self, states: &ui::States) -> Option<&App> {
    //     states.infra.app_list.list.selected()
    //         .map(|index| self.apps.get(index))?
    // }
}

