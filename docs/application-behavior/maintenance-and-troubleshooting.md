# Maintenance and troubleshooting map

## Updating this reference

For any user-visible behavior change:

1. Read the relevant workflow document before editing code.
2. Identify the state and persistence boundary affected by the change.
3. Update UI behavior, native validation, storage behavior, and tests together
   when the behavior crosses those layers.
4. Update the relevant document in this folder in the same change.
5. Add a new row or distinction to a behavior matrix when data survives,
   aggregates, resets, or deletes differently than another similarly named
   value.
6. Keep wording suitable for later adaptation into a user troubleshooting
   guide: describe expected behavior, failure symptoms, and supported recovery.

Do not copy volatile implementation details into every document. Prefer stable
user concepts, then link to the owning code and tests here.

## Source and test map

| Area | UI behavior | Native/storage behavior | Primary automated coverage |
| --- | --- | --- | --- |
| AI workspace and key buckets | `apps/desktop/src/shared/AiWorkspace.tsx` | `apps/desktop/src-tauri/src/ai_keys.rs` | `apps/desktop/tests/ai-workspace.test.tsx`, `apps/desktop/src-tauri/src/ai_keys_tests.rs` |
| Key card models and limits | `AiKeyCustomization.tsx`, `AiGeneralSpending.tsx` | `ai_settings.rs`, `crates/ort-storage/src/ai_activity.rs` | desktop UI tests, `ai_settings` tests, `ai_activity` tests |
| Data selector and graph | `AiDataKeyPicker.tsx`, `AiUsageChart.tsx` | `ai_settings.rs`, `ai_activity.rs` | `ai-workspace.test.tsx`, chart tests, storage activity tests |
| Export and clear-by-month | `AiDataActionDialog.tsx` | `ai_settings.rs`, `ai_activity.rs` | desktop UI tests and activity-range storage tests |
| Removed-key data deletion | `AiRemovedKeyDataDialog.tsx`, `AiWorkspace.tsx` | `ai_keys.rs`, `ai_activity.rs` | desktop workflow test, removed-key authorization test, per-key clearing test |
| Resume Edit/View workflow | `apps/desktop/src/shared/App.tsx` | resume commands in `apps/desktop/src-tauri/src/lib.rs` and storage crates | desktop editor/workspace tests and resume storage tests |
| Resume state/autosave | `editor-state.ts`, `resume-editor.ts` | save/publish commands and revisioned storage | editor state, contracts, and storage tests |
| PDF/DOCX presentation and export | `PdfPreview.tsx`, `document-styles.ts` | `pdf_preview.rs`, `text_export.rs`, `ort-render`, `ort-documents` | preview, render, PDF, and DOCX tests |
| Import | `DocumentImport.tsx`, `ImportReviewFlow.tsx` | `import_review/`, parser worker crates | import review/session/storage/parser tests |
| Backup and recovery | `BackupPanel.tsx` | `backup_export.rs`, `ort-backup`, storage recovery | backup and restore tests |
| Local data deletion | `StoragePanel.tsx` | `data_deletion.rs`, storage deletion | data deletion and native storage tests |

## Troubleshooting questions to answer first

### AI key or request issue

1. Is there an active key, or is the Active key bucket empty?
2. Is the key paused, removal-failed, or successfully removed?
3. Does its provider/preset have an enabled catalog model?
4. Has either its per-key cap or the general cap run out of available
   exposure?
5. Is another test or request still active?
6. Did the secure vault mutation succeed, fail visibly, or have an uncertain
   cleanup outcome?

### Data total or graph issue

1. Which key scope, metric, currency, and timeframe is selected?
2. Is the user comparing retained graph activity with durable lifetime spend?
3. Was activity cleared by month, removed by retention, or permanently deleted
   for a removed key?
4. Does the point report partial or unknown usage?
5. Is All keys expected to change because a component key's retained activity
   changed?

### Resume save, publish, or export issue

1. Is the user in Edit or View, and which Saved/Published source is selected?
2. Are there blocking validation messages?
3. Is the draft dirty, saved, or in a save-interrupted state?
4. Is Publish disabled because content is unsaved or already published?
5. Which style is currently selected, and was the PDF preview generated with a
   different style or revision?
6. Did the native Save dialog complete, cancel, or return an uncertain result?

## Terminology

- **Draft / saved resume**: mutable latest saved master-resume revision.
- **Published resume / snapshot**: immutable saved content revision selected by
  Publish.
- **Active key**: sole key used for ordinary AI requests.
- **Removed key**: credential successfully removed from My Keys while its Data
  history may remain.
- **Removal failed**: cleanup did not fully confirm; key remains visible,
  paused, and retryable.
- **Activity**: retained operation/attempt rows used by Data graphs and JSON
  export.
- **Lifetime spend**: durable estimated accounting that activity cleanup does
  not reduce.
- **Cap exposure**: counted plus reserved plus unresolved cost since the cap's
  current baseline.
- **All keys**: aggregation of currently retained activity, not a separately
  maintained historical total.

