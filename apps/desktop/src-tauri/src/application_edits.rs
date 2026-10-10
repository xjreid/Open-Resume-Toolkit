//! Reviewed workspace edits and PDF preflight run as one background job.
#![allow(clippy::needless_pass_by_value)]
use crate::application_exports::ApplicationExportState;
use crate::application_materials::error;
use crate::background_work::preflight_pdf;
use crate::{DesktopState, storage_failure, window_not_authorized};
use ort_application::application_workspace::{ensure_profile, load, save_reviewed};
use ort_domain::{
    ApplicationWorkspace, CommandResponse, MaterialKind, ResumeDocument, SavedWorkspace,
};
use ort_storage::StorageError;
use tauri::{Manager, WebviewWindow};

#[tauri::command]
pub async fn save_application_workspace(
    window: WebviewWindow,
    expected_profile_id: uuid::Uuid,
    expected_revision: i64,
    workspace: ApplicationWorkspace,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    crate::background_work::run(move || {
        save_application_workspace_blocking(
            window,
            expected_profile_id,
            expected_revision,
            workspace,
        )
    })
    .await
}

fn save_application_workspace_blocking(
    window: WebviewWindow,
    expected_profile_id: uuid::Uuid,
    expected_revision: i64,
    workspace: ApplicationWorkspace,
) -> CommandResponse<SavedWorkspace> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let state = window.state::<DesktopState>();
    let prior = match state.with_store(|store| {
        ensure_profile(store, expected_profile_id)?;
        load(store)?.ok_or(StorageError::NotFound)
    }) {
        Ok(prior) => prior,
        Err(problem) => return storage_failure(&problem),
    };
    if prior.revision != expected_revision {
        return storage_failure(&StorageError::RevisionConflict);
    }
    let resume_pdf_unchanged = workspace.style == prior.workspace.style
        && printable_resume_unchanged(&prior.workspace.resume, &workspace.resume);
    if !resume_pdf_unchanged
        && let Err(code) = preflight_pdf(expected_profile_id, &workspace, MaterialKind::Resume)
    {
        return error(code);
    }
    if (workspace.cover_letter != prior.workspace.cover_letter
        || workspace.resume.contact != prior.workspace.resume.contact
        || workspace.style != prior.workspace.style)
        && workspace.cover_letter.is_some()
        && let Err(code) = preflight_pdf(expected_profile_id, &workspace, MaterialKind::CoverLetter)
    {
        return error(code);
    }
    match state.with_store(|store| {
        ensure_profile(store, expected_profile_id)?;
        save_reviewed(store, expected_revision, &workspace)
    }) {
        Ok(saved) => {
            if resume_pdf_unchanged {
                window.state::<ApplicationExportState>().promote_unchanged(
                    expected_profile_id,
                    prior.revision,
                    saved.revision,
                    MaterialKind::Resume,
                );
            }
            CommandResponse::success(saved)
        }
        Err(problem) => storage_failure(&problem),
    }
}

// Empty sections are useful while editing but contribute nothing to either
// exported format. Adding one should not force a PDF rerender or invalidate an
// already prepared download of the same printable content.
pub(crate) fn printable_resume_unchanged(before: &ResumeDocument, after: &ResumeDocument) -> bool {
    let mut before = before.clone();
    let mut after = after.clone();
    before
        .sections
        .retain(|section| !section.entries.is_empty());
    after.sections.retain(|section| !section.entries.is_empty());
    before == after
}
