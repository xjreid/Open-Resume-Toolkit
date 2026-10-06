use crate::{ApprovedAnswer, DocumentStyle, EntityId, ResumeDocument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleInfo {
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub location: String,
}

impl RoleInfo {
    #[must_use]
    pub fn valid(&self) -> bool {
        [&self.company, &self.title, &self.location]
            .into_iter()
            .all(|value| value.chars().count() <= 200)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    NotFound,
    ConfirmedMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertCategory {
    DegreeLevel,
    FieldOfStudy,
    GraduationDate,
    CertificationOrProfessionalLicense,
    NamedSkillOrTechnology,
    LanguageProficiency,
    ExperienceDuration,
    PortfolioOrWorkSample,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlertEvidence {
    pub field_id: EntityId,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualificationAlert {
    pub id: String,
    pub kind: AlertKind,
    pub category: AlertCategory,
    pub requirement: String,
    // Older saved workspaces did not retain the validated qualification target.
    #[serde(default)]
    pub target: String,
    pub job_excerpt: String,
    pub job_start: usize,
    pub job_end: usize,
    pub mandatory_reason: String,
    pub resume_evidence: Option<AlertEvidence>,
    pub validation_version: u16,
    pub published_revision: i64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationWorkspace {
    pub schema_version: u16,
    pub published_revision: i64,
    pub job_description: String,
    #[serde(default)]
    pub job_url: String,
    #[serde(default)]
    pub role_info: RoleInfo,
    pub resume: ResumeDocument,
    pub change_points: Vec<String>,
    pub alerts: Vec<QualificationAlert>,
    pub alerts_truncated: bool,
    pub dismissed_alert_ids: Vec<String>,
    pub ignore_all_alerts: bool,
    pub cover_letter: Option<String>,
    pub question: String,
    pub answer: String,
    pub approved_answers: Vec<ApprovedAnswer>,
    pub style: DocumentStyle,
}

#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedWorkspace {
    pub revision: i64,
    pub workspace: ApplicationWorkspace,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageOneDraft {
    pub job_description: String,
    pub job_url: String,
    pub style: DocumentStyle,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedStageOneDraft {
    pub revision: i64,
    pub draft: StageOneDraft,
}

#[must_use]
pub fn valid_application_url(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    if value.len() > 4_096 {
        return false;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && !url.query_pairs().any(|(key, _)| {
            let lower = key.to_ascii_lowercase();
            lower.starts_with("utm_")
                || matches!(
                    lower.as_str(),
                    "token"
                        | "access_token"
                        | "auth"
                        | "session"
                        | "code"
                        | "utm_source"
                        | "utm_medium"
                        | "utm_campaign"
                        | "utm_term"
                        | "utm_content"
                        | "gclid"
                        | "fbclid"
                        | "msclkid"
                        | "mc_cid"
                        | "mc_eid"
                )
        })
}

pub const APPLICATION_SCHEMA_VERSION: u16 = 1;
pub const APPLICATION_WORKSPACE_KEY: &str = "application.workspace.v1";
pub const APPLICATION_STAGE_ONE_KEY: &str = "application.stage1.v1";
pub const APPLICATION_CAPTURE_KEY: &str = "application.capture.pending.v1";
pub const MAX_CAPTURE_TEXT_BYTES: usize = 128 * 1024;
/// Persistence policies belong to the typed record, not to a generic storage key switch.
#[must_use]
pub fn application_record_size_limit(key: &str) -> Option<usize> {
    match key {
        APPLICATION_WORKSPACE_KEY => Some(1024 * 1024),
        APPLICATION_STAGE_ONE_KEY | APPLICATION_CAPTURE_KEY => Some(256 * 1024),
        _ => None,
    }
}
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaterialKind {
    Resume,
    CoverLetter,
}
