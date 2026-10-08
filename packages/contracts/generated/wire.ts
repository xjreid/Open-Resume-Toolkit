// @generated from native JSON schemas and command signatures.
export type AddAiKeyRequest = {
  apiKey: string;
  name?: string | null;
  provider: string;
};
export type AiActivityMonth = {
  fromUnixMs: number;
  label: string;
  toUnixMs: number;
};
export type AiAttemptFailure = {
  attemptId: string;
  callNumber: number;
  category: string | null;
  details: AiFailureDetails | null;
  durationMs: number | null;
  effectiveModel: string | null;
  operationId: string;
  operationType: string;
  provider: string;
  requestedModel: string;
  startedAtUnixMs: number;
  usage: Usage | null;
  usageComplete: boolean;
};
export type AiCapActionRequest = { credentialId: string; period: AiPeriod };
export type AiCapPolicySummary = {
  activatedAtUnixMs: number;
  countedMicros: number;
  credentialId: string;
  currency: string;
  limitMicros: number;
  period: AiPeriod;
  periodEndUnixMs: number | null;
  periodStartUnixMs: number;
  reservedMicros: number;
  revision: number;
  timeZone: string;
  unresolvedMicros: number;
};
export type AiFailureDetails = {
  code: string;
  finishReason: string | null;
  httpStatus: number | null;
  pageCount: number | null;
  providerReason: string | null;
  validationIssues: Array<string>;
};
export type AiGeneralSettings = {
  cap: AiCapPolicySummary | null;
  lifetimeSpendByCurrencyMicros: Record<string, number>;
  lifetimeSpendPartial: boolean;
};
export type AiKeyAction = "select_primary" | "pause" | "unpause" | "remove";
export type AiKeyRegistry = {
  keys: Array<SavedAiKey>;
  primaryCredentialId: string | null;
};
export type AiKeySettings = {
  cap: AiCapPolicySummary | null;
  lifetimeSpendByCurrencyMicros: Record<string, number>;
  lifetimeSpendPartial: boolean;
};
export type AiMonitoringBucket = {
  attempts: number;
  costByCurrencyMicros: Record<string, number>;
  label: string;
  partial: boolean;
  totalTokens: number;
  unknownCount: number;
  usage: Usage;
};
export type AiMonitoringSummary = {
  attempts: number;
  byCredentialId: Record<string, number>;
  byModel: Record<string, number>;
  byOperationType: Record<string, number>;
  byPreset: Record<string, number>;
  byProvider: Record<string, number>;
  byStatus: Record<string, number>;
  costByCurrencyMicros: Record<string, number>;
  currency: string | null;
  estimatedCostMicros: number;
  logicalOperations: number;
  partial: boolean;
  recentFailures: Array<AiAttemptFailure>;
  timeBuckets: Array<AiMonitoringBucket>;
  totalTokens: number;
  unknownCount: number;
  unresolvedReservedMicros: number;
  usage: Usage;
};
export type AiPeriod = "week" | "month" | "year" | "all_time";
export type AiProgress = { kind: string; text: string };
export type AiRetentionSummary = { policy: string; removedOperations: number };
export type AiTestPreview = {
  credentialId: string;
  currency: string;
  estimatedInputTokens: number;
  maximumCostMicros: number;
  model: string;
  provider: string;
};
export type AiTestResult = {
  attemptId: string;
  confirmed: boolean;
  effectiveModel: string;
  estimatedCostMicros: number | null;
  usage: Usage;
  usageComplete: boolean;
};
export type AlertCategory =
  | "degree_level"
  | "field_of_study"
  | "graduation_date"
  | "certification_or_professional_license"
  | "named_skill_or_technology"
  | "language_proficiency"
  | "experience_duration"
  | "portfolio_or_work_sample";
export type AlertEvidence = { fieldId: string; value: string };
export type AlertKind = "not_found" | "confirmed_mismatch";
export type ApplicationContext = {
  aiBusy: boolean;
  aiLabel: string;
  aiReady: boolean;
  browserConnected: boolean;
  model: string | null;
  modelOptions: Array<ApplicationModelOption>;
  profileId: string;
  publishedRevision: number | null;
  selectedKeyId: string | null;
  selectedKeyReady: boolean;
};
export type ApplicationExportFormat = "pdf" | "docx";
export type ApplicationModelOption = { model: string };
export type ApplicationPopupKind =
  | "job"
  | "url"
  | "resume-view"
  | "resume-edit"
  | "cover"
  | "cover-view";
export type ApplicationWorkspace = {
  alerts: Array<QualificationAlert>;
  alertsTruncated: boolean;
  answer: string;
  approvedAnswers: Array<ApprovedAnswer>;
  changePoints: Array<string>;
  changeSummary?: Array<string>;
  coverLetter: string | null;
  dismissedAlertIds: Array<string>;
  ignoreAllAlerts: boolean;
  jobDescription: string;
  jobUrl: string;
  publishedRevision: number;
  question: string;
  resume: ResumeDocument;
  roleInfo: RoleInfo;
  schemaVersion: number;
  style: DocumentStyle;
  trackerMetadata?: TrackerMetadata | null;
};
export type ApplyImportReviewPayload = {
  decisionsJson: string;
  reviewId: string;
};
export type ApplyImportReviewRequest = {
  contractVersion: number;
  payload: ApplyImportReviewPayload;
  requestId: string;
};
export type ApprovedAnswer = { answer: string; question: string };
export type BackupRecoveryStatusPayload = Record<string, never>;
export type BackupRecoveryStatusRequest = {
  contractVersion: number;
  payload: BackupRecoveryStatusPayload;
  requestId: string;
};
export type BackupRecoveryStatusResponse = {
  restartOperationPending: boolean;
  safetyCleanupPending: boolean;
  safetyCopyAvailable: boolean;
};
export type BeginImportPayload = { expectedRevision?: number | null };
export type BeginImportRequest = {
  contractVersion: number;
  payload: BeginImportPayload;
  requestId: string;
};
export type Bullet = { id: string; order: number; text: string };
export type CalendarDate = {
  expected: boolean;
  month: number | null;
  year: number;
};
export type CaptureEnvelope = {
  kind: string;
  payload: CapturePayload;
  protocolVersion: number;
  requestId: string;
  sentAt: string;
};
export type CapturePayload = {
  browser: string;
  target: string;
  text: string;
  title: string;
  url: string;
};
export type CaptureStatus = {
  error: string | null;
  phase: string;
  sessionId: string | null;
};
export type Catalog = {
  catalogId: string;
  entries: Array<CatalogEntry>;
  expiresAt: string;
  formatVersion: number;
  issuedAt: string;
  minimumAppVersion: string;
};
export type CatalogEntry = {
  currency: string;
  disabled: boolean;
  effectiveFrom: string;
  effectiveTo: string | null;
  maxInputTokens: number;
  maxOutputTokens: number;
  model: string;
  operations: Array<OperationType>;
  preset: Preset;
  prices: Array<Price>;
  provider: Provider;
  source: string;
  verifiedAt: string;
};
export type ChangeAiKeyRequest = { action: AiKeyAction; credentialId: string };
export type CloseDecision = "cancel" | "quit";
export type CloseStatusRequest = {
  contractVersion: number;
  payload: EmptyPayload;
  requestId: string;
};
export type CloseStatusResponse = { pendingAttempt: string | null };
export type ConnectionStatus = { available: boolean; connected: boolean };
export type ContactDetails = {
  email: string;
  fullName: string;
  links: Array<Link>;
  location: string;
  phone: string;
};
export type CredentialProvider = "openai" | "anthropic" | "gemini";
export type DateEnd =
  | { kind: "present" }
  | { kind: "date"; value: CalendarDate };
export type DeleteAllLocalDataPayload = { confirmation: string };
export type DeleteAllLocalDataRequest = {
  contractVersion: number;
  payload: DeleteAllLocalDataPayload;
  requestId: string;
};
export type DeleteAllLocalDataResponse =
  | { freshProfileReady: boolean; status: "deleted" }
  | { restartRequired: boolean; status: "cleanup_pending" };
export type DeleteRemovedKeyDataRequest = { credentialIds: Array<string> };
export type DeleteRemovedKeyDataResult = {
  clearedOperations: number;
  registry: AiKeyRegistry;
};
export type DeleteSafetyCopyRequest = {
  contractVersion: number;
  payload: SafetyCopyActionPayload;
  requestId: string;
};
export type DeleteSafetyCopyResponse = { deleted: boolean };
export type DocumentStyle = "plain" | "technical" | "professional" | "modern";
export type EmptyPayload = Record<string, never>;
export type ErrorEnvelope = {
  code: string;
  details: Record<string, unknown>;
  messageKey: string;
  operationId?: string | null;
  retryable: boolean;
};
export type ExportBackupPayload = { passphrase: string };
export type ExportBackupRequest = {
  contractVersion: number;
  payload: ExportBackupPayload;
  requestId: string;
};
export type ExportBackupResponse =
  | { status: "cancelled" }
  | {
      byteCount: number;
      cleanupPending: boolean;
      durabilityUnconfirmed: boolean;
      formatMajor: number;
      formatMinor: number;
      status: "exported";
    };
export type ExportDocxRequest = {
  contractVersion: number;
  payload: StyledExportPayload;
  requestId: string;
};
export type ExportDocxResponse =
  | { status: "cancelled" }
  | {
      byteCount: number;
      cleanupPending: boolean;
      durabilityUnconfirmed: boolean;
      formatVersion: number;
      revision: number;
      source: ExportSource;
      status: "exported";
      templateId?: string | null;
    };
export type ExportSource = "saved_draft" | "published_snapshot";
export type ExportTextPayload = {
  expectedRevision: number;
  source: ExportSource;
};
export type ExportTextRequest = {
  contractVersion: number;
  payload: ExportTextPayload;
  requestId: string;
};
export type ExportTextResponse =
  | { status: "cancelled" }
  | {
      byteCount: number;
      cleanupPending: boolean;
      durabilityUnconfirmed: boolean;
      formatVersion: number;
      revision: number;
      source: ExportSource;
      status: "exported";
    };
export type FinishSelection = { entry: TrackerEntry };
export type HealthPayload = Record<string, never>;
export type HealthRequest = {
  contractVersion: number;
  payload: HealthPayload;
  requestId: string;
};
export type HealthResponse = {
  appVersion: string;
  contractVersion: number;
  profile: RuntimeProfile;
  status: HealthStatus;
  storageStatus: StorageStatus;
};
export type HealthStatus = "ok";
export type ImportChoice =
  | { kind: "pending" }
  | { kind: "reject" }
  | { heading: string; kind: "section"; target: ImportSectionTarget }
  | {
      field: ImportContactField;
      kind: "contact";
      mode: ImportContactMode;
      value: string;
    }
  | { bullet: boolean; kind: "text"; target: ImportTextTarget; text: string };
export type ImportChoices = { choices: Array<ImportChoice> };
export type ImportContactField = "fullName" | "email" | "phone" | "location";
export type ImportContactMode = "fillEmpty" | "replace" | "keepExisting";
export type ImportReviewBlock = {
  explanation: string;
  page: number;
  proposedSection: number | null;
  source: string;
  suggestedTarget: ImportReviewTarget;
  suggestedValue: string;
};
export type ImportReviewContacts = {
  email: string;
  fullName: string;
  location: string;
  phone: string;
};
export type ImportReviewPayload = { reviewId: string };
export type ImportReviewRequest = {
  contractVersion: number;
  payload: ImportReviewPayload;
  requestId: string;
};
export type ImportReviewSection = { heading: string; id: string };
export type ImportReviewSnapshot = {
  baseRevision: number;
  blocks: Array<ImportReviewBlock>;
  contacts: ImportReviewContacts;
  id: string;
  importedDocument: ResumeDocument;
  mappingVersion: number;
  sections: Array<ImportReviewSection>;
};
export type ImportReviewTarget =
  | "section"
  | "text"
  | "bullet"
  | "fullName"
  | "email"
  | "phone"
  | "location";
export type ImportSectionTarget =
  | { kind: "new" }
  | { id: string; kind: "existing" };
export type ImportTextTarget =
  | { index: number; kind: "proposed" }
  | { id: string; kind: "existing" }
  | { heading: string; kind: "new" };
export type Link = { id?: string; label: string; order?: number; url: string };
export type ListKind = "skills" | "coursework";
export type LoadResumeRequest = {
  contractVersion: number;
  payload: EmptyPayload;
  requestId: string;
};
export type MapImportReviewPayload = { documentJson: string; reviewId: string };
export type MapImportReviewRequest = {
  contractVersion: number;
  payload: MapImportReviewPayload;
  requestId: string;
};
export type MaterialKind = "resume" | "cover_letter";
export type MaterialPdf = { base64: string; filename: string };
export type NamedField = {
  id: string;
  isSkill: boolean;
  label: string;
  listKind?: ListKind;
  order: number;
  value: string;
};
export type OpenPortablePdfHistoryPayload = { passphrase: string };
export type OpenPortablePdfHistoryRequest = {
  contractVersion: number;
  payload: OpenPortablePdfHistoryPayload;
  requestId: string;
};
export type OperationType =
  | "tailor_resume"
  | "refine_resume"
  | "cover_letter"
  | "answer_question"
  | "import_mapping"
  | "credential_test";
export type PdfExportResponse =
  | { status: "cancelled" }
  | {
      byteCount: number;
      cleanupPending: boolean;
      durabilityUnconfirmed: boolean;
      pdfSha256: string;
      renderId: string;
      status: "exported";
    };
export type PdfPreviewResponse = {
  generatedAtUnixMs: number;
  pdfBase64: string;
  receipt: PdfRenderReceipt;
  renderId: string;
  revision: number;
  source: ExportSource;
};
export type PdfRegeneratePayload = { manifestId: string; style: DocumentStyle };
export type PdfRegenerateRequest = {
  contractVersion: number;
  payload: PdfRegeneratePayload;
  requestId: string;
};
export type PdfReleaseResponse = { released: boolean };
export type PdfRenderHistoryRequest = {
  contractVersion: number;
  payload: EmptyPayload;
  requestId: string;
};
export type PdfRenderHistoryResponse = { manifests: Array<PdfRenderManifest> };
export type PdfRenderManifest = {
  generatedAtUnixMs: number;
  lastGeneratedAtUnixMs: number;
  manifestId: string;
  receipt: PdfRenderReceipt;
  renderCount: number;
  source: ExportSource;
  sourceRevision: number;
};
export type PdfRenderReceipt = {
  byteCount: number;
  documentSchemaVersion: number;
  documentSha256: string;
  fontBundleId: string;
  fontBundleSha256: string;
  pageCount: number;
  pdfSha256: string;
  rendererVersion: string;
  templateId: string;
  templateSha256: string;
};
export type PdfReplayPayload = { manifestId: string };
export type PdfReplayRequest = {
  contractVersion: number;
  payload: PdfReplayPayload;
  requestId: string;
};
export type PdfReplayResponse = {
  accessibleText: string;
  preview: PdfPreviewResponse;
};
export type PdfTicketPayload = { renderId: string };
export type PdfTicketRequest = {
  contractVersion: number;
  payload: PdfTicketPayload;
  requestId: string;
};
export type PortablePdfArchivePayload = { archiveId: string };
export type PortablePdfArchiveRequest = {
  contractVersion: number;
  payload: PortablePdfArchivePayload;
  requestId: string;
};
export type PortablePdfHistoryResponse =
  | { status: "cancelled" }
  | {
      archiveId: string;
      expiresAtUnixMs: number;
      incompatibleReceipts: number;
      manifests: Array<PdfRenderManifest>;
      status: "opened";
      totalManifests: number;
      unavailableSources: number;
    };
export type PortablePdfRegeneratePayload = {
  archiveId: string;
  manifestId: string;
  style: DocumentStyle;
};
export type PortablePdfRegenerateRequest = {
  contractVersion: number;
  payload: PortablePdfRegeneratePayload;
  requestId: string;
};
export type PortablePdfReplayPayload = {
  archiveId: string;
  manifestId: string;
};
export type PortablePdfReplayRequest = {
  contractVersion: number;
  payload: PortablePdfReplayPayload;
  requestId: string;
};
export type PreparedApplicationExport = {
  docxReady: boolean;
  pageCount: number;
  pdfReady: boolean;
  revision: number;
};
export type Preset = "economy" | "balanced" | "quality";
export type Price = { category: PriceCategory; microsPerMillion: number };
export type PriceCategory =
  | "input"
  | "cached_input"
  | "cache_write"
  | "output"
  | "reasoning";
export type Provider = "open_ai" | "anthropic" | "gemini";
export type PublishResumePayload = { expectedDraftRevision: number };
export type PublishResumeRequest = {
  contractVersion: number;
  payload: PublishResumePayload;
  requestId: string;
};
export type PublishResumeResponse = {
  draftRevision: number;
  published: VersionedResumeResponse;
};
export type QualificationAlert = {
  category: AlertCategory;
  id: string;
  jobEnd: number;
  jobExcerpt: string;
  jobStart: number;
  kind: AlertKind;
  mandatoryReason: string;
  publishedRevision: number;
  requirement: string;
  resumeEvidence: AlertEvidence | null;
  target: string;
  validationVersion: number;
};
export type RenameAiKeyRequest = { credentialId: string; name: string };
export type ResolveClosePayload = { attempt: string; decision: CloseDecision };
export type ResolveCloseRequest = {
  contractVersion: number;
  payload: ResolveClosePayload;
  requestId: string;
};
export type RestoreBackupPayload = { confirmation: string; passphrase: string };
export type RestoreBackupRequest = {
  contractVersion: number;
  payload: RestoreBackupPayload;
  requestId: string;
};
export type RestoreBackupResponse =
  | { status: "cancelled" }
  | { restartRequired: boolean; safetyCopyRetained: boolean; status: "staged" };
export type ResumeDate = {
  end: DateEnd | null;
  id: string;
  label: string;
  order: number;
  start: CalendarDate | null;
};
export type ResumeDocument = {
  contact: ContactDetails;
  documentId: string;
  schemaVersion: number;
  sections: Array<ResumeSection>;
  title: string;
};
export type ResumeEntry = {
  bullets: Array<Bullet>;
  dateRange: string;
  dates?: Array<ResumeDate>;
  fields: Array<NamedField>;
  heading: string;
  id: string;
  links: Array<Link>;
  location: string;
  order: number;
  subheading: string;
};
export type ResumeSection = {
  entries: Array<ResumeEntry>;
  heading: string;
  id: string;
  order: number;
};
export type ResumeWorkspaceResponse = {
  draft: VersionedResumeResponse | null;
  latestPublished: VersionedResumeResponse | null;
};
export type RoleInfo = { company: string; location: string; title: string };
export type RollbackSafetyCopyRequest = {
  contractVersion: number;
  payload: SafetyCopyActionPayload;
  requestId: string;
};
export type RollbackSafetyCopyResponse = {
  currentProfileRetained: boolean;
  restartRequired: boolean;
};
export type RuntimeProfile = "development";
export type SafetyCopyActionPayload = { confirmation: string };
export type SaveAiCapRequest = {
  credentialId: string;
  expectedRevision?: number | null;
  limitMicros: number;
  period: AiPeriod;
  timeZone: string;
};
export type SaveResumePayload = {
  document: ResumeDocument;
  expectedRevision?: number | null;
};
export type SaveResumeRequest = {
  contractVersion: number;
  payload: SaveResumePayload;
  requestId: string;
};
export type SavedAiKey = {
  cleanupRequired: boolean;
  createdAt: string;
  credentialId: string;
  model: string;
  name?: string | null;
  paused: boolean;
  provider: CredentialProvider;
  removed: boolean;
};
export type SavedPendingCapture = {
  capture: CaptureEnvelope;
  revision: number;
};
export type SavedStageOneDraft = { draft: StageOneDraft; revision: number };
export type SavedTrackerEntry = {
  id: string;
  revision: number;
  value: TrackerEntry;
};
export type SavedWorkspace = {
  revision: number;
  workspace: ApplicationWorkspace;
};
export type SetAiKeyModelRequest = { credentialId: string; model: string };
export type StageOneDraft = {
  jobDescription: string;
  jobUrl: string;
  style: DocumentStyle;
};
export type StorageStatus = "ready" | "development_gated" | "unavailable";
export type StorageUsagePayload = Record<string, never>;
export type StorageUsageRequest = {
  contractVersion: number;
  payload: StorageUsagePayload;
  requestId: string;
};
export type StorageUsageResponse = {
  databaseBytes: number;
  databaseSchema: number;
  diagnosticEvents: number;
  drafts: number;
  manifestBytes: number;
  publishedSnapshots: number;
  recoveryMetadataBytes: number;
  renderManifests: number;
  settings: number;
  sharedMemoryBytes: number;
  totalProfileBytes: number;
  trackerEntries: number;
  walBytes: number;
};
export type StyledExportPayload = {
  expectedRevision: number;
  source: ExportSource;
  style?: DocumentStyle;
};
export type StyledExportRequest = {
  contractVersion: number;
  payload: StyledExportPayload;
  requestId: string;
};
export type TrackerEntry = {
  answers: Array<ApprovedAnswer>;
  company: string;
  coverContact: ContactDetails | null;
  coverLetter: string | null;
  customStatus: string;
  dateApplied: string;
  location: string;
  resume: ResumeDocument | null;
  sourceUrl: string;
  status: string;
  style: DocumentStyle;
  title: string;
};
export type TrackerMetadata = {
  company: string;
  customStatus: string;
  dateApplied: string;
  location: string;
  sourceUrl: string;
  status: string;
  title: string;
};
export type TrackerRecord = { id: string; revision: number; value: unknown };
export type TrackerSummary = {
  answerCount: number;
  hasCoverLetter: boolean;
  hasResume: boolean;
  id: string;
  revision: number;
  value: TrackerMetadata;
};
export type Usage = {
  cacheWriteTokens: number;
  cachedInputTokens: number;
  inputTokens: number;
  outputTokens: number;
  reasoningTokens: number;
};
export type ValidateBackupPayload = { passphrase: string };
export type ValidateBackupRequest = {
  contractVersion: number;
  payload: ValidateBackupPayload;
  requestId: string;
};
export type ValidateBackupResponse =
  | { status: "cancelled" }
  | {
      aiAttempts: number;
      aiOperations: number;
      appVersion: string;
      byteCount: number;
      createdAt: string;
      databaseSchema: number;
      documentSchema: number;
      formatMajor: number;
      formatMinor: number;
      masterDrafts: number;
      publishedResumes: number;
      renderManifests: number;
      settings: number;
      status: "validated";
    };
export type VersionedResumeResponse = {
  document: ResumeDocument;
  revision: number;
};
export type DesktopCommands = {
  add_ai_key: { args: { request: AddAiKeyRequest }; value: AiKeyRegistry };
  application_capture_status: {
    args: Record<string, never>;
    value: CaptureStatus;
  };
  application_context: {
    args: Record<string, never>;
    value: ApplicationContext;
  };
  application_overlay_visibility: {
    args: Record<string, never>;
    value: boolean;
  };
  apply_application_job_capture: {
    args: { expectedRevision?: number | null; requestId: string };
    value: SavedStageOneDraft;
  };
  browser_connection_status: {
    args: Record<string, never>;
    value: ConnectionStatus;
  };
  cancel_ai_test: { args: Record<string, never>; value: boolean };
  cancel_application_capture: {
    args: { sessionId: string };
    value: CaptureStatus;
  };
  cancel_application_generation: {
    args: Record<string, never>;
    value: boolean;
  };
  change_ai_key: {
    args: { request: ChangeAiKeyRequest };
    value: AiKeyRegistry;
  };
  clear_ai_monitoring: {
    args: {
      credentialIds?: Array<string> | null;
      months: Array<AiActivityMonth>;
    };
    value: number;
  };
  clear_ai_primary: { args: Record<string, never>; value: AiKeyRegistry };
  close_status: {
    args: { request: CloseStatusRequest };
    value: CloseStatusResponse;
  };
  connect_development_browser: { args: Record<string, never>; value: boolean };
  delete_removed_ai_key_data: {
    args: { request: DeleteRemovedKeyDataRequest };
    value: DeleteRemovedKeyDataResult;
  };
  delete_safety_copy: {
    args: { request: DeleteSafetyCopyRequest };
    value: DeleteSafetyCopyResponse;
  };
  delete_tracker_entry: {
    args: { expectedRevision: number; id: string };
    value: boolean;
  };
  disable_ai_cap: { args: { request: AiCapActionRequest }; value: boolean };
  disable_ai_general_cap: { args: Record<string, never>; value: boolean };
  disconnect_development_browser: {
    args: Record<string, never>;
    value: boolean;
  };
  download_application_export: {
    args: {
      expectedRevision: number;
      format: ApplicationExportFormat;
      kind: MaterialKind;
    };
    value: boolean;
  };
  drag_application_export: {
    args: {
      expectedRevision: number;
      format: ApplicationExportFormat;
      kind: MaterialKind;
    };
    value: boolean;
  };
  export_ai_monitoring: {
    args: {
      credentialIds?: Array<string> | null;
      months: Array<AiActivityMonth>;
      timeZone: string;
    };
    value: string;
  };
  export_portable_backup: {
    args: { request: ExportBackupRequest };
    value: ExportBackupResponse;
  };
  finish_application: {
    args: { expectedRevision: number; selection?: FinishSelection | null };
    value: boolean;
  };
  generate_application_answer: {
    args: { expectedRevision: number; question: string };
    value: SavedWorkspace;
  };
  generate_application_cover_letter: {
    args: { expectedRevision: number; instruction: string };
    value: SavedWorkspace;
  };
  get_tracker_entry: { args: { id: string }; value: SavedTrackerEntry };
  health: { args: { request: HealthRequest }; value: HealthResponse };
  hide_application_popup: { args: Record<string, never>; value: boolean };
  list_tracker_entries: {
    args: {
      limit?: number | null;
      offset?: number | null;
      search?: string | null;
      status?: string | null;
    };
    value: Array<TrackerSummary>;
  };
  load_ai_caps: {
    args: { credentialId?: string | null };
    value: Array<AiCapPolicySummary>;
  };
  load_ai_catalog: { args: Record<string, never>; value: Catalog };
  load_ai_connection: { args: Record<string, never>; value: AiKeyRegistry };
  load_ai_general_settings: {
    args: Record<string, never>;
    value: AiGeneralSettings;
  };
  load_ai_key_settings: {
    args: { credentialId: string };
    value: AiKeySettings;
  };
  load_ai_monitoring: {
    args: {
      bucketSize: string;
      credentialId?: string | null;
      fromUnixMs: number;
      timeZone: string;
      toUnixMs: number;
    };
    value: AiMonitoringSummary;
  };
  load_ai_retention: { args: Record<string, never>; value: AiRetentionSummary };
  load_application_capture: {
    args: Record<string, never>;
    value: SavedPendingCapture | null;
  };
  load_application_stage_one: {
    args: Record<string, never>;
    value: SavedStageOneDraft | null;
  };
  load_application_workspace: {
    args: Record<string, never>;
    value: SavedWorkspace | null;
  };
  load_backup_recovery_status: {
    args: { request: BackupRecoveryStatusRequest };
    value: BackupRecoveryStatusResponse;
  };
  load_resume: {
    args: { request: LoadResumeRequest };
    value: ResumeWorkspaceResponse;
  };
  load_storage_usage: {
    args: { request: StorageUsageRequest };
    value: StorageUsageResponse;
  };
  open_tracker_link: { args: { target: string }; value: boolean };
  prepare_application_exports: {
    args: { expectedRevision: number; kind: MaterialKind };
    value: PreparedApplicationExport;
  };
  preview_ai_test: { args: { credentialId: string }; value: AiTestPreview };
  preview_application_pdf: {
    args: { expectedRevision: number; kind: MaterialKind };
    value: PdfPreviewResponse;
  };
  preview_tracker_pdf: {
    args: { expectedRevision: number; id: string; kind: MaterialKind };
    value: MaterialPdf;
  };
  publish_resume: {
    args: { request: PublishResumeRequest };
    value: PublishResumeResponse;
  };
  refine_application_answer: {
    args: { expectedRevision: number; instruction: string };
    value: SavedWorkspace;
  };
  regenerate_application_resume: {
    args: { correctionInstruction: string; expectedRevision: number };
    value: SavedWorkspace;
  };
  rename_ai_key: {
    args: { request: RenameAiKeyRequest };
    value: AiKeyRegistry;
  };
  request_application_capture: {
    args: { target?: string | null };
    value: CaptureStatus;
  };
  reset_ai_cap: { args: { request: AiCapActionRequest }; value: boolean };
  reset_ai_general_cap: { args: Record<string, never>; value: boolean };
  resolve_application_capture: {
    args: {
      accept: boolean;
      expectedRevision?: number | null;
      requestId: string;
      reviewedText?: string | null;
      reviewedUrl?: string | null;
    };
    value: boolean;
  };
  resolve_close: {
    args: { request: ResolveCloseRequest };
    value: CloseStatusResponse;
  };
  restore_portable_backup: {
    args: { request: RestoreBackupRequest };
    value: RestoreBackupResponse;
  };
  retry_storage: { args: Record<string, never>; value: boolean };
  rollback_safety_copy: {
    args: { request: RollbackSafetyCopyRequest };
    value: RollbackSafetyCopyResponse;
  };
  save_ai_cap: {
    args: { request: SaveAiCapRequest };
    value: AiCapPolicySummary;
  };
  save_ai_general_cap: {
    args: {
      expectedRevision?: number | null;
      limitMicros: number;
      timeZone: string;
    };
    value: AiCapPolicySummary;
  };
  save_ai_retention: { args: { policy: string }; value: AiRetentionSummary };
  save_application_stage_one: {
    args: {
      draft: StageOneDraft;
      expectedProfileId: string;
      expectedRevision?: number | null;
    };
    value: SavedStageOneDraft;
  };
  save_application_workspace: {
    args: {
      expectedProfileId: string;
      expectedRevision: number;
      workspace: ApplicationWorkspace;
    };
    value: SavedWorkspace;
  };
  save_resume: {
    args: { request: SaveResumeRequest };
    value: VersionedResumeResponse;
  };
  save_tracker_entry: {
    args: {
      entry: TrackerEntry;
      expectedRevision?: number | null;
      id?: string | null;
    };
    value: TrackerSummary;
  };
  save_tracker_metadata: {
    args: { expectedRevision: number; id: string; metadata: TrackerMetadata };
    value: TrackerSummary;
  };
  set_ai_key_model: {
    args: { request: SetAiKeyModelRequest };
    value: AiKeyRegistry;
  };
  show_application_popup: {
    args: { kind: ApplicationPopupKind };
    value: boolean;
  };
  start_application: {
    args: { jobDescription: string; jobUrl: string; style: DocumentStyle };
    value: SavedWorkspace;
  };
  test_ai_connection: {
    args: {
      credentialId: string;
      expectedMaximumCostMicros: number;
      expectedModel: string;
      onProgress: unknown;
    };
    value: AiTestResult;
  };
  toggle_application_overlay: { args: Record<string, never>; value: boolean };
  validate_portable_backup: {
    args: { request: ValidateBackupRequest };
    value: ValidateBackupResponse;
  };
};
