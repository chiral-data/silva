//! JSON Schemas for `workflow.toml` and `job.toml`, generated from the structs.
//!
//! The committed copies under `schema/` are pinned to this output by the tests
//! below, so a struct change that skips regeneration fails `cargo test`.

use crate::job::JobMeta;
use crate::workflow::{SCHEMA_VERSION, WorkflowMeta};

/// Schema for `.chiral/workflow.toml`.
pub fn workflow_schema() -> String {
    render(schemars::schema_for!(WorkflowMeta))
}

/// Schema for `.chiral/job.toml`.
pub fn job_schema() -> String {
    render(schemars::schema_for!(JobMeta))
}

fn render(mut schema: schemars::Schema) -> String {
    schema.insert("x-schema-version".into(), SCHEMA_VERSION.into());
    let mut out = serde_json::to_string_pretty(&schema).expect("a schema always serializes");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_committed(generated: &str, kind: &str) {
        let path = format!(
            "{}/../schema/{kind}.schema.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == generated,
            "schema/{kind}.schema.json is stale; regenerate it with:\n  \
             cargo run -q -p silva -- schema {kind} > schema/{kind}.schema.json"
        );
    }

    #[test]
    fn committed_workflow_schema_is_current() {
        assert_committed(&workflow_schema(), "workflow");
    }

    #[test]
    fn committed_job_schema_is_current() {
        assert_committed(&job_schema(), "job");
    }

    #[test]
    fn workflow_schema_is_stamped_and_constrains_version() {
        let schema: serde_json::Value = serde_json::from_str(&workflow_schema()).unwrap();
        assert_eq!(schema["x-schema-version"], SCHEMA_VERSION);
        assert_eq!(
            schema["properties"]["schema_version"]["pattern"],
            r"^\d+\.\d+$"
        );
    }
}
