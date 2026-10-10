use ort_domain::{
    ApplicationWorkspace, Bullet, DocumentLimits, EntityId, MaterialKind, NamedField,
    ResumeDocument, ResumeEntry, ResumeSection,
};
use ort_storage::StorageError;
/// Builds an export document directly from retained content.
/// # Errors
/// Rejects a missing cover letter or an invalid source document.
pub fn document_for(
    resume: &ResumeDocument,
    cover_letter: Option<&str>,
    kind: MaterialKind,
) -> Result<ResumeDocument, StorageError> {
    match kind {
        MaterialKind::Resume => Ok(resume.clone()),
        MaterialKind::CoverLetter => {
            let text = cover_letter.ok_or(StorageError::NotFound)?;
            let mut document = ResumeDocument::empty("Cover letter");
            // Stable derived identities let preflight and export share the exact document.
            let id = |slot: u128| {
                EntityId::parse(
                    &uuid::Uuid::from_u128(resume.document_id.as_uuid().as_u128() ^ slot)
                        .to_string(),
                )
                .map_err(|_| StorageError::InvalidData)
            };
            document.document_id = id(1)?;
            document.schema_version = resume.schema_version;
            document.contact = resume.contact.clone();
            document.sections = vec![ResumeSection {
                id: id(2)?,
                order: 0,
                heading: "Cover letter".into(),
                entries: vec![ResumeEntry {
                    id: id(3)?,
                    order: 0,
                    heading: String::new(),
                    subheading: String::new(),
                    date_range: String::new(),
                    dates: (document.schema_version == 2).then(Vec::new),
                    location: String::new(),
                    fields: vec![NamedField {
                        id: id(4)?,
                        order: 0,
                        label: ort_domain::PARAGRAPH_FIELD_LABEL.into(),
                        value: text.to_owned(),
                        is_skill: false,
                        list_kind: None,
                    }],
                    bullets: Vec::<Bullet>::new(),
                    links: Vec::new(),
                }],
            }];
            document
                .validate(DocumentLimits::default())
                .map_err(|_| StorageError::InvalidData)?;
            Ok(document)
        }
    }
}

/// Checks renderer bounds before persisting a reviewed edit.
/// # Errors
/// Returns a stable error code for missing content or renderer bounds.
pub fn preflight_pdf(
    workspace: &ApplicationWorkspace,
    kind: MaterialKind,
) -> Result<(), &'static str> {
    let document = document_for(&workspace.resume, workspace.cover_letter.as_deref(), kind)
        .map_err(|_| "PDF_UNAVAILABLE")?;
    document
        .validate(DocumentLimits::default())
        .map_err(|_| "RESUME_INVALID")?;
    ort_render::render_pdf_with_style(&document, workspace.style)
        .map(|_| ())
        .map_err(|_| "PDF_UNAVAILABLE")
}
