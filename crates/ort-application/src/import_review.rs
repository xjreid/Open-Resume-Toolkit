//! In-memory review only. The caller must commit the returned save payload
//! through storage's optimistic-revision transaction after explicit review.
//! No constructor starts a parser, writes a file, or changes a saved draft.

use std::collections::HashMap;

use ort_documents::import::{ContactField, ImportProposal, ProposedContent, section_kind};
use ort_domain::{
    Bullet, ContactDetails, DocumentLimits, EntityId, NamedField, ResumeDocument, ResumeEntry,
    ResumeSection, SaveResumePayload, VersionedResumeResponse,
};

/// All editable decision strings combined, in Unicode scalar values. Source
/// text is independently capped at 50,000 characters. This is not a draft limit.
pub const MAX_REVIEW_CHARACTERS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionTarget {
    New,
    Existing(EntityId),
}

#[derive(Clone, PartialEq, Eq)]
pub enum TextTarget {
    /// Index of an explicitly accepted section proposal, not a worker ID.
    ProposedSection(usize),
    ExistingSection(EntityId),
    /// Equal new headings share one new section within this review only.
    /// Existing draft sections are never selected implicitly by heading.
    NewSection(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactMode {
    FillEmpty,
    Replace,
    KeepExisting,
}

/// An explicit decision, with editable values. Raw source stays available in
/// the proposal even after a value is edited or rejected. Nothing is accepted
/// by default. Deliberate reclassification/moving is allowed during review.
#[derive(Clone, PartialEq, Eq)]
pub enum ReviewDecision {
    Reject,
    Section {
        heading: String,
        target: SectionTarget,
    },
    Contact {
        field: ContactField,
        value: String,
        mode: ContactMode,
    },
    Text {
        text: String,
        is_bullet: bool,
        target: TextTarget,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReviewError {
    #[error("the base draft or reviewed content is invalid or exceeds document limits")]
    InvalidContent,
    #[error("every source block requires an explicit review decision")]
    IncompleteReview,
    #[error("the review item does not exist")]
    UnknownItem,
    #[error("the draft changed; restart review against the current saved revision")]
    StaleDraft,
    #[error("the requested section is absent or was not accepted")]
    MissingDestination,
    #[error("choose whether to replace or keep the existing contact value")]
    ContactConflict,
}

pub struct ImportReview {
    base: VersionedResumeResponse,
    proposal: ImportProposal,
    decisions: Vec<Option<ReviewDecision>>,
}

impl ImportReview {
    /// Creates a review bound to one exact saved draft, with no accepted items.
    ///
    /// # Errors
    /// Rejects an invalid base document or unsaved revision.
    pub fn new(
        base: VersionedResumeResponse,
        proposal: ImportProposal,
    ) -> Result<Self, ReviewError> {
        if base.revision < 1 || base.document.validate(DocumentLimits::default()).is_err() {
            return Err(ReviewError::InvalidContent);
        }
        let decisions = vec![None; proposal.items().len()];
        Ok(Self {
            base,
            proposal,
            decisions,
        })
    }

    #[must_use]
    pub fn proposal(&self) -> &ImportProposal {
        &self.proposal
    }

    #[must_use]
    pub fn decision(&self, index: usize) -> Option<&ReviewDecision> {
        self.decisions.get(index).and_then(Option::as_ref)
    }

    /// Records/replaces one explicit decision, without mutating the base draft.
    /// Validation of the complete candidate happens at preparation time.
    ///
    /// # Errors
    /// Rejects out-of-range indices; the caller cannot add unattached content.
    pub fn decide(&mut self, index: usize, decision: ReviewDecision) -> Result<(), ReviewError> {
        if index >= self.decisions.len() {
            return Err(ReviewError::UnknownItem);
        }
        let total = self
            .decisions
            .iter()
            .enumerate()
            .filter(|(position, _)| *position != index)
            .filter_map(|(_, value)| value.as_ref())
            .map(decision_characters)
            .sum::<usize>()
            + decision_characters(&decision);
        if total > MAX_REVIEW_CHARACTERS {
            return Err(ReviewError::InvalidContent);
        }
        let slot = self
            .decisions
            .get_mut(index)
            .ok_or(ReviewError::UnknownItem)?;
        *slot = Some(decision);
        Ok(())
    }

    /// Removes a decision when a user returns an item to pending review.
    ///
    /// # Errors
    /// Rejects unknown item indices.
    pub fn reset_decision(&mut self, index: usize) -> Result<(), ReviewError> {
        *self
            .decisions
            .get_mut(index)
            .ok_or(ReviewError::UnknownItem)? = None;
        Ok(())
    }

    /// A suggestion for display only: calling this does NOT accept an item.
    #[must_use]
    pub fn suggested_decision(&self, index: usize) -> Option<ReviewDecision> {
        let item = self.proposal.items().get(index)?;
        Some(match &item.content {
            ProposedContent::Section { heading, .. } => ReviewDecision::Section {
                heading: heading.clone(),
                target: SectionTarget::New,
            },
            ProposedContent::Contact { field, value } => ReviewDecision::Contact {
                field: *field,
                value: value.clone(),
                mode: ContactMode::FillEmpty,
            },
            ProposedContent::Text { text, is_bullet } => ReviewDecision::Text {
                text: text.clone(),
                is_bullet: *is_bullet,
                target: item.section_index.map_or_else(
                    || TextTarget::NewSection("Imported text".to_owned()),
                    TextTarget::ProposedSection,
                ),
            },
        })
    }

    /// Possible same-type/heading destinations for a section. These are review
    /// hints, never automatic merges or proof of duplicate factual content.
    #[must_use]
    pub fn possible_section_duplicates(&self, index: usize) -> Vec<EntityId> {
        let Some(ProposedContent::Section { heading, kind }) =
            self.proposal.items().get(index).map(|item| &item.content)
        else {
            return vec![];
        };
        self.base
            .document
            .sections
            .iter()
            .filter(|section| {
                heading.trim().to_lowercase() == section.heading.trim().to_lowercase()
                    || (kind.is_some() && *kind == section_kind(&section.heading))
            })
            .map(|section| section.id)
            .collect()
    }

    #[must_use]
    pub fn contact_conflicts(&self, index: usize) -> bool {
        match self.proposal.items().get(index).map(|item| &item.content) {
            Some(ProposedContent::Contact { field, .. }) => {
                !contact_value(&self.base.document.contact, *field)
                    .trim()
                    .is_empty()
            }
            _ => false,
        }
    }

    /// Prepares an entirely validated candidate without a partial draft write.
    /// Preparation can be retried after editing decisions; it does not consume
    /// the source. The eventual storage save MUST use `expected_revision` (CAS)
    /// and retire the review only after confirmed success. This is not a commit.
    ///
    /// # Errors
    /// Rejects stale drafts, incomplete review, missing destinations, contact
    /// conflicts, and invalid/oversized output without truncating any source.
    pub fn prepare(
        &self,
        current: &VersionedResumeResponse,
    ) -> Result<SaveResumePayload, ReviewError> {
        if &self.base != current {
            return Err(ReviewError::StaleDraft);
        }
        let decisions = self
            .decisions
            .iter()
            .map(|item| item.as_ref().ok_or(ReviewError::IncompleteReview))
            .collect::<Result<Vec<_>, _>>()?;
        let mut document = self.base.document.clone();
        let mut sections = HashMap::new();
        let mut new_text_sections = HashMap::new();
        for (index, decision) in decisions.iter().enumerate() {
            if let ReviewDecision::Section { heading, target } = decision {
                validate_text(heading)?;
                let id = match target {
                    SectionTarget::New => append_section(&mut document, heading)?,
                    SectionTarget::Existing(id) => {
                        // Choosing merge does not rename or replace existing content.
                        require_existing(&self.base.document, *id)?;
                        *id
                    }
                };
                sections.insert(index, id);
            }
        }
        for decision in decisions {
            match decision {
                ReviewDecision::Reject | ReviewDecision::Section { .. } => {}
                ReviewDecision::Contact { field, value, mode } => {
                    apply_contact(&mut document.contact, *field, value, *mode)?;
                }
                ReviewDecision::Text {
                    text,
                    is_bullet,
                    target,
                } => {
                    validate_text(text)?;
                    let id = resolve_text_target(
                        target,
                        &self.base.document,
                        &mut document,
                        &sections,
                        &mut new_text_sections,
                    )?;
                    append_text(&mut document, id, text, *is_bullet)?;
                }
            }
        }
        document
            .validate(DocumentLimits::default())
            .map_err(|_| ReviewError::InvalidContent)?;
        Ok(SaveResumePayload {
            expected_revision: Some(self.base.revision),
            document,
        })
    }
}

impl std::fmt::Debug for ImportReview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportReview")
            .field("base_revision", &self.base.revision)
            .field("items", &self.decisions.len())
            .field(
                "decided",
                &self.decisions.iter().filter(|d| d.is_some()).count(),
            )
            .finish_non_exhaustive()
    }
}

fn validate_text(text: &str) -> Result<(), ReviewError> {
    if text.trim().is_empty()
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        Err(ReviewError::InvalidContent)
    } else {
        Ok(())
    }
}

fn require_existing(document: &ResumeDocument, id: EntityId) -> Result<(), ReviewError> {
    if document.sections.iter().any(|section| section.id == id) {
        Ok(())
    } else {
        Err(ReviewError::MissingDestination)
    }
}

fn append_section(document: &mut ResumeDocument, heading: &str) -> Result<EntityId, ReviewError> {
    validate_text(heading)?;
    if document.sections.len() >= DocumentLimits::default().sections {
        return Err(ReviewError::InvalidContent);
    }
    let id = EntityId::new();
    document.sections.push(ResumeSection {
        id,
        order: u16::try_from(document.sections.len()).map_err(|_| ReviewError::InvalidContent)?,
        heading: heading.to_owned(),
        entries: vec![],
    });
    Ok(id)
}

fn resolve_text_target(
    target: &TextTarget,
    base: &ResumeDocument,
    document: &mut ResumeDocument,
    sections: &HashMap<usize, EntityId>,
    new_sections: &mut HashMap<String, EntityId>,
) -> Result<EntityId, ReviewError> {
    match target {
        TextTarget::ProposedSection(index) => sections
            .get(index)
            .copied()
            .ok_or(ReviewError::MissingDestination),
        TextTarget::ExistingSection(id) => {
            require_existing(base, *id)?;
            Ok(*id)
        }
        TextTarget::NewSection(heading) => {
            if let Some(id) = new_sections.get(heading) {
                return Ok(*id);
            }
            let id = append_section(document, heading)?;
            new_sections.insert(heading.clone(), id);
            Ok(id)
        }
    }
}

fn append_text(
    document: &mut ResumeDocument,
    id: EntityId,
    text: &str,
    is_bullet: bool,
) -> Result<(), ReviewError> {
    let section = document
        .sections
        .iter_mut()
        .find(|section| section.id == id)
        .ok_or(ReviewError::MissingDestination)?;
    let mut entry = ResumeEntry {
        id: EntityId::new(),
        order: u16::try_from(section.entries.len()).map_err(|_| ReviewError::InvalidContent)?,
        heading: String::new(),
        subheading: String::new(),
        date_range: String::new(),
        location: String::new(),
        fields: vec![],
        bullets: vec![],
        links: vec![],
    };
    if is_bullet {
        entry.bullets.push(Bullet {
            id: EntityId::new(),
            order: 0,
            text: text.to_owned(),
        });
    } else {
        entry.fields.push(NamedField {
            id: EntityId::new(),
            order: 0,
            label: String::new(),
            value: text.to_owned(),
            is_skill: false,
        });
    }
    section.entries.push(entry);
    Ok(())
}

fn contact_value(contact: &ContactDetails, field: ContactField) -> &str {
    match field {
        ContactField::FullName => &contact.full_name,
        ContactField::Email => &contact.email,
        ContactField::Phone => &contact.phone,
        ContactField::Location => &contact.location,
    }
}

fn apply_contact(
    contact: &mut ContactDetails,
    field: ContactField,
    value: &str,
    mode: ContactMode,
) -> Result<(), ReviewError> {
    if mode == ContactMode::KeepExisting {
        return Ok(());
    }
    validate_text(value)?;
    if mode == ContactMode::FillEmpty && !contact_value(contact, field).trim().is_empty() {
        return Err(ReviewError::ContactConflict);
    }
    value.clone_into(match field {
        ContactField::FullName => &mut contact.full_name,
        ContactField::Email => &mut contact.email,
        ContactField::Phone => &mut contact.phone,
        ContactField::Location => &mut contact.location,
    });
    Ok(())
}

fn decision_characters(decision: &ReviewDecision) -> usize {
    match decision {
        ReviewDecision::Reject => 0,
        ReviewDecision::Section { heading, .. } => heading.chars().count(),
        ReviewDecision::Contact { value, .. } => value.chars().count(),
        ReviewDecision::Text { text, target, .. } => {
            text.chars().count()
                + match target {
                    TextTarget::NewSection(heading) => heading.chars().count(),
                    _ => 0,
                }
        }
    }
}
