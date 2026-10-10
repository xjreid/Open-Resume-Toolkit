use super::*;
use crate::application_edits::printable_resume_unchanged;
use crate::application_exports::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use ort_application::application_workspace::save_reviewed;
use ort_documents::render_docx_with_style;
use ort_domain::DocumentLimits;
use ort_domain::{Bullet, EntityId, NamedField, ResumeEntry, ResumeSection, RoleInfo};
use ort_vault::testing::MemoryDatabaseKeyVault;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

fn workspace() -> ApplicationWorkspace {
    ApplicationWorkspace {
        schema_version: SCHEMA_VERSION,
        tracker_metadata: None,
        published_revision: 1,
        job_description: "Rust required".into(),
        job_url: String::new(),
        role_info: RoleInfo {
            company: "Example Company".into(),
            title: "Engineer".into(),
            location: "Remote".into(),
        },
        resume: ResumeDocument::empty("Resume"),
        change_points: vec!["Kept the published resume content.".into()],
        change_summary: Vec::new(),
        alerts: vec![],
        alerts_truncated: false,
        dismissed_alert_ids: vec![],
        ignore_all_alerts: false,
        cover_letter: Some(
            "Dear Hiring Team,\n\nI am writing about this role.\n\nSincerely,\nApplicant".into(),
        ),
        question: String::new(),
        answer: String::new(),
        approved_answers: vec![],
        style: DocumentStyle::Technical,
    }
}

#[path = "application_materials_tests/artifacts.rs"]
mod artifacts;

#[path = "application_materials_tests/capture.rs"]
mod capture;

#[path = "application_materials_tests/workflow.rs"]
mod workflow;
