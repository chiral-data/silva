use std::path::Path;

use serde::Deserialize;

use crate::data_model::provider;

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Files {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// scripts to be executed in sequence 
    pub scripts: Vec<String>,
}
 
impl Files {
    pub fn all_files(&self) -> Vec<String> {
        [
            &["@job.toml", "@pre.sh", "@post.sh", "Dockerfile", "\n", "[inputs]"].into_iter().map(|s| s.to_string()).collect(),
            &self.inputs,
            &vec!["\n", "[scripts]"].into_iter().map(String::from).collect(),
            &self.scripts,
            &vec!["\n", "[outputs]"].into_iter().map(String::from).collect(),
            &self.outputs,
            // &vec!["[build files]", "Dockerfile", "@run.sh"].into_iter().map(String::from).collect() // file for building docker image
        ]
        .iter()
        .flat_map(|v| v.iter().map(|s| s.to_string()).collect::<Vec<String>>())
        .collect()
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Settings {
    pub files: Files,
    pub dok: Option<provider::sakura_internet::DokSettings>,
}

impl Settings {
    pub fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }

    pub fn new_from_file(filepath: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(filepath)?;
        Self::new(&content)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_file() {
        let toml_str = r#"
        [files]
        inputs = [
            "a.in",
            "b.in",
            "c.in"
        ]
        outputs = [
            "1.out",
        ]
        scripts = [
            "start.sh",
            "finish.sh"
        ]

        [dok]
        base_image = "a"
        extra_build_commands = ["python load_model.py"]
        http_path = "/dok"
        http_port = 11203
        "#;
        let s = Settings::new(toml_str).unwrap();
        assert_eq!(s.files.inputs.len(), 3);
        assert_eq!(s.files.outputs.len(), 1);
        assert_eq!(s.files.scripts.len(), 2);
        let dok = s.dok.unwrap();
        // assert_eq!(dok.base_image, "a");
        // assert_eq!(dok.extra_build_commands.unwrap().len(), 1);
        assert_eq!(dok.http_path.unwrap(), "/dok");
        assert_eq!(dok.http_port.unwrap(), 11203);
    }
}

