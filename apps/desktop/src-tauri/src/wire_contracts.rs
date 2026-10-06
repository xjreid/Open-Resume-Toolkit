//! Serialized contracts for development tooling; command models remain with their owners.
use crate::{
    ai_keys, ai_request, ai_settings, application_exports, application_materials, browser_bridge,
    tracker,
};
/// Development tooling uses exactly the schemas serialized by native commands.
/// Canonical serialized desktop wire schemas used by the contract generator.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "declarative schema registry, without workflow branches"
)]
pub fn desktop_wire_schemas() -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut schemas = std::collections::BTreeMap::new();
    macro_rules! register {
        ($name:literal, $ty:ty) => {
            let settings = schemars::generate::SchemaSettings::default();
            let settings = if $name.ends_with("Request") {
                settings.for_deserialize()
            } else {
                settings.for_serialize()
            };
            schemas.insert(
                $name.into(),
                serde_json::to_value(settings.into_generator().into_root_schema_for::<$ty>())
                    .expect("schema is serializable"),
            );
        };
    }
    register!("AddAiKeyRequest", ai_keys::AddAiKeyRequest);
    register!("AiKeyAction", ai_keys::AiKeyAction);
    register!("AiKeyRegistry", ai_keys::AiKeyRegistry);
    register!("ChangeAiKeyRequest", ai_keys::ChangeAiKeyRequest);
    register!(
        "DeleteRemovedKeyDataRequest",
        ai_keys::DeleteRemovedKeyDataRequest
    );
    register!(
        "DeleteRemovedKeyDataResult",
        ai_keys::DeleteRemovedKeyDataResult
    );
    register!("RenameAiKeyRequest", ai_keys::RenameAiKeyRequest);
    register!("SavedAiKey", ai_keys::SavedAiKey);
    register!("SetAiKeyPresetRequest", ai_keys::SetAiKeyPresetRequest);
    register!("AiProgress", ai_request::AiProgress);
    register!("AiTestPreview", ai_request::AiTestPreview);
    register!("AiTestResult", ai_request::AiTestResult);
    register!("AiActivityMonth", ai_settings::AiActivityMonth);
    register!("AiCapActionRequest", ai_settings::AiCapActionRequest);
    register!("AiGeneralSettings", ai_settings::AiGeneralSettings);
    register!("AiKeySettings", ai_settings::AiKeySettings);
    register!("AiRetentionSummary", ai_settings::AiRetentionSummary);
    register!("SaveAiCapRequest", ai_settings::SaveAiCapRequest);
    register!(
        "ApplicationExportFormat",
        application_exports::ApplicationExportFormat
    );
    register!(
        "ApplicationPresetOption",
        application_materials::ApplicationPresetOption
    );
    register!("MaterialPdf", application_exports::MaterialPdf);
    register!(
        "PreparedApplicationExport",
        application_exports::PreparedApplicationExport
    );
    register!(
        "SavedPendingCapture",
        application_materials::SavedPendingCapture
    );
    register!("ConnectionStatus", browser_bridge::ConnectionStatus);
    register!("Catalog", ort_ai::Catalog);
    register!("ApplicationWorkspace", ort_domain::ApplicationWorkspace);
    register!(
        "ApplyImportReviewRequest",
        ort_domain::ApplyImportReviewRequest
    );
    register!("BeginImportRequest", ort_domain::BeginImportRequest);
    register!("CloseDecision", ort_domain::CloseDecision);
    register!("CloseStatusResponse", ort_domain::CloseStatusResponse);
    register!(
        "DeleteAllLocalDataRequest",
        ort_domain::DeleteAllLocalDataRequest
    );
    register!(
        "DeleteAllLocalDataResponse",
        ort_domain::DeleteAllLocalDataResponse
    );
    register!("DocumentStyle", ort_domain::DocumentStyle);
    register!("ErrorEnvelope", ort_domain::ErrorEnvelope);
    register!("ExportDocxRequest", ort_domain::ExportDocxRequest);
    register!("ExportDocxResponse", ort_domain::ExportDocxResponse);
    register!("ExportSource", ort_domain::ExportSource);
    register!("ExportTextRequest", ort_domain::ExportTextRequest);
    register!("ExportTextResponse", ort_domain::ExportTextResponse);
    register!("HealthRequest", ort_domain::HealthRequest);
    register!("HealthResponse", ort_domain::HealthResponse);
    register!("ImportChoices", ort_domain::ImportChoices);
    register!("ImportReviewRequest", ort_domain::ImportReviewRequest);
    register!("ImportReviewSnapshot", ort_domain::ImportReviewSnapshot);
    register!("LoadResumeRequest", ort_domain::LoadResumeRequest);
    register!("MapImportReviewRequest", ort_domain::MapImportReviewRequest);
    register!("MaterialKind", ort_domain::MaterialKind);
    register!(
        "OpenPortablePdfHistoryRequest",
        ort_domain::OpenPortablePdfHistoryRequest
    );
    register!("PdfExportResponse", ort_domain::PdfExportResponse);
    register!("PdfPreviewResponse", ort_domain::PdfPreviewResponse);
    register!("PdfRegenerateRequest", ort_domain::PdfRegenerateRequest);
    register!(
        "PdfRenderHistoryResponse",
        ort_domain::PdfRenderHistoryResponse
    );
    register!("PdfRenderManifest", ort_domain::PdfRenderManifest);
    register!("PdfRenderReceipt", ort_domain::PdfRenderReceipt);
    register!("PdfReplayRequest", ort_domain::PdfReplayRequest);
    register!(
        "PortablePdfArchiveRequest",
        ort_domain::PortablePdfArchiveRequest
    );
    register!(
        "PortablePdfHistoryResponse",
        ort_domain::PortablePdfHistoryResponse
    );
    register!(
        "PortablePdfRegenerateRequest",
        ort_domain::PortablePdfRegenerateRequest
    );
    register!(
        "PortablePdfReplayRequest",
        ort_domain::PortablePdfReplayRequest
    );
    register!("PublishResumeRequest", ort_domain::PublishResumeRequest);
    register!("PublishResumeResponse", ort_domain::PublishResumeResponse);
    register!("ResolveCloseRequest", ort_domain::ResolveCloseRequest);
    register!("ResumeDocument", ort_domain::ResumeDocument);
    register!(
        "ResumeWorkspaceResponse",
        ort_domain::ResumeWorkspaceResponse
    );
    register!("SaveResumeRequest", ort_domain::SaveResumeRequest);
    register!("SavedStageOneDraft", ort_domain::SavedStageOneDraft);
    register!("SavedWorkspace", ort_domain::SavedWorkspace);
    register!("StageOneDraft", ort_domain::StageOneDraft);
    register!("StorageUsageRequest", ort_domain::StorageUsageRequest);
    register!("StorageUsageResponse", ort_domain::StorageUsageResponse);
    register!("StyledExportRequest", ort_domain::StyledExportRequest);
    register!("TrackerEntry", ort_domain::TrackerEntry);
    register!(
        "VersionedResumeResponse",
        ort_domain::VersionedResumeResponse
    );
    register!("CaptureStatus", ort_ipc::capture_session::CaptureStatus);
    register!(
        "AiCapPolicySummary",
        ort_storage::ai_activity::AiCapPolicySummary
    );
    register!(
        "AiMonitoringSummary",
        ort_storage::ai_activity::AiMonitoringSummary
    );
    register!("TrackerRecord", ort_storage::tracker::TrackerRecord);
    register!("FinishSelection", tracker::FinishSelection);
    register!(
        "ApplicationContext",
        application_materials::ApplicationContext
    );
    register!("SavedTrackerEntry", tracker::SavedTrackerEntry);
    register!("TrackerSummary", ort_storage::tracker::TrackerSummary);
    register!("CloseStatusRequest", ort_domain::CloseStatusRequest);
    register!("ApplicationPopupKind", crate::ApplicationPopupKind);
    schemas
}

/// Synthetic payloads serialized by the same Rust types returned by commands.
/// # Panics
/// Panics if a fixed development fixture or its serialization is invalid.
#[must_use]
pub fn desktop_wire_fixtures() -> std::collections::BTreeMap<String, serde_json::Value> {
    use ort_domain::{
        ApplicationWorkspace, DocumentStyle, ResumeDocument, RoleInfo, SavedWorkspace, TrackerEntry,
    };
    use serde_json::to_value;
    let mut resume = ResumeDocument::empty("Wire fixture");
    // Fixed synthetic identity keeps regeneration deterministic.
    resume.document_id =
        serde_json::from_str("\"019a0000-0000-7000-8000-000000000001\"").expect("fixed fixture ID");
    let workspace = ApplicationWorkspace {
        schema_version: 1,
        tracker_metadata: None,
        published_revision: 1,
        job_description: "Synthetic job".into(),
        job_url: String::new(),
        role_info: RoleInfo::default(),
        resume: resume.clone(),
        change_points: vec![],
        alerts: vec![],
        alerts_truncated: false,
        dismissed_alert_ids: vec![],
        ignore_all_alerts: false,
        cover_letter: None,
        question: String::new(),
        answer: String::new(),
        approved_answers: vec![],
        style: DocumentStyle::Technical,
    };
    let tracker = TrackerEntry {
        company: "Synthetic".into(),
        title: "Engineer".into(),
        location: String::new(),
        date_applied: "2026-10-06".into(),
        status: "applied".into(),
        custom_status: String::new(),
        source_url: String::new(),
        resume: Some(resume),
        cover_letter: None,
        cover_contact: None,
        answers: vec![],
        style: DocumentStyle::Technical,
    };
    std::collections::BTreeMap::from([
        (
            "export_portable_backup".into(),
            to_value(ort_domain::ExportBackupResponse::Exported {
                byte_count: 1000,
                format_major: 1,
                format_minor: ort_backup::FORMAT_MINOR,
                cleanup_pending: false,
                durability_unconfirmed: false,
            })
            .expect("fixture serializes"),
        ),
        (
            "validate_portable_backup".into(),
            to_value(ort_domain::ValidateBackupResponse::Validated {
                byte_count: 1000,
                format_major: 1,
                format_minor: ort_backup::FORMAT_MINOR,
                app_version: "0.0.0-dev".into(),
                database_schema: 5,
                document_schema: 2,
                created_at: "2026-10-06T12:00:00Z".into(),
                master_drafts: 1,
                published_resumes: 101,
                settings: 0,
                render_manifests: 0,
                ai_operations: 10_001,
                ai_attempts: 20_001,
            })
            .expect("fixture serializes"),
        ),
        (
            "load_ai_connection".into(),
            to_value(ai_keys::AiKeyRegistry::default()).expect("fixture serializes"),
        ),
        (
            "load_ai_monitoring".into(),
            to_value(ort_storage::ai_activity::AiMonitoringSummary::default())
                .expect("fixture serializes"),
        ),
        (
            "load_application_workspace".into(),
            to_value(SavedWorkspace {
                revision: 1,
                workspace,
            })
            .expect("fixture serializes"),
        ),
        (
            "get_tracker_entry".into(),
            to_value(tracker::SavedTrackerEntry {
                id: "019a0000-0000-7000-8000-000000000002".into(),
                revision: 1,
                value: tracker,
            })
            .expect("fixture serializes"),
        ),
    ])
}
