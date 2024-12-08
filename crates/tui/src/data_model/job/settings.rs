use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub input_files: Vec<String>,
    pub output_files: Vec<String>,
    /// scripts to be executed in sequence 
    pub script_files: Vec<String>
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
        input_files = [
            "a.in",
            "b.in",
            "c.in"
        ]
        output_files = [
            "1.out",
        ]
        script_files = [
            "start.sh",
            "finish.sh"
        ]
        "#;
        
        let s = Settings::new(toml_str).unwrap();
        assert_eq!(s.input_files.len(), 3);
        assert_eq!(s.output_files.len(), 1);
        assert_eq!(s.script_files.len(), 2);
    }
}

