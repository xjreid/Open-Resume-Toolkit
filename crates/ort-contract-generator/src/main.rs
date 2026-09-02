use std::{fs, path::PathBuf};

use ort_domain::{
    HealthRequest, HealthResponse, LoadResumeRequest, PublishResumeRequest, PublishResumeResponse,
    ResumeWorkspaceResponse, SaveResumeRequest, VersionedResumeResponse,
};
use schemars::schema_for;

const TYPESCRIPT: &str = include_str!("health.ts.template");
const RESUME_TYPESCRIPT: &str = include_str!("resume.ts.template");
const COMPATIBILITY: &str = include_str!("compatibility.json.template");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let output = root.join("packages/contracts/generated");
    fs::create_dir_all(&output)?;

    write_schema::<HealthRequest>(&output.join("health.request.schema.json"))?;
    write_schema::<HealthResponse>(&output.join("health.response.schema.json"))?;
    write_schema::<LoadResumeRequest>(&output.join("resume.load.request.schema.json"))?;
    write_schema::<SaveResumeRequest>(&output.join("resume.save.request.schema.json"))?;
    write_schema::<PublishResumeRequest>(&output.join("resume.publish.request.schema.json"))?;
    write_schema::<ResumeWorkspaceResponse>(&output.join("resume.workspace.response.schema.json"))?;
    write_schema::<VersionedResumeResponse>(&output.join("resume.versioned.response.schema.json"))?;
    write_schema::<PublishResumeResponse>(&output.join("resume.publish.response.schema.json"))?;
    fs::write(output.join("health.ts"), TYPESCRIPT)?;
    fs::write(output.join("resume.ts"), RESUME_TYPESCRIPT)?;
    fs::write(output.join("compatibility.json"), COMPATIBILITY)?;

    println!("Generated development contracts in {}", output.display());
    Ok(())
}

fn write_schema<T: schemars::JsonSchema>(
    destination: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema_for!(T);
    let schema_json = format!("{}\n", serde_json::to_string_pretty(&schema)?);
    fs::write(destination, schema_json)?;
    Ok(())
}

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(std::path::Path::parent)
        .map(PathBuf::from)
        .ok_or_else(|| "contract generator is not inside the workspace".into())
}
