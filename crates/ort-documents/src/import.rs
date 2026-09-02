//! Parent-side extraction validation and deterministic proposals, not a parser.
//! No binary file, path, worker launch, network, or database authority lives here.

use ort_domain::DocumentLimits;
use serde::Deserialize;

pub const EXTRACTION_VERSION: u16 = 1;
pub const MAPPING_VERSION: u16 = 1;
pub const MAX_EXTRACTION_BYTES: usize = 512 * 1024;
pub const MAX_EXTRACTED_CHARACTERS: usize = 50_000;
pub const MAX_BLOCK_CHARACTERS: usize = 30_000;
pub const MAX_BLOCKS: usize = 1_000;
pub const MAX_PAGES: u16 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputFormat {
    Pdf,
    Docx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Heading,
    Paragraph,
    ListItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExtractionWire {
    version: u16,
    format: InputFormat,
    page_count: u16,
    blocks: Vec<ExtractedBlock>,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractedBlock {
    pub page: u16,
    pub kind: BlockKind,
    pub text: String,
}

/// Constructible only through the bounded parent-side decoder. Block indices
/// are assigned locally in wire order, not accepted as worker-owned authority.
pub struct ValidatedExtraction {
    format: InputFormat,
    page_count: u16,
    blocks: Vec<ExtractedBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("extraction exceeds its configured limit")]
    LimitExceeded,
    #[error("extraction protocol is invalid or unsupported")]
    InvalidExtraction,
    #[error("extraction contains unsupported control characters")]
    UnsupportedControl,
    #[error("no readable text was extracted; OCR is not available")]
    NoReadableText,
}

impl ValidatedExtraction {
    /// Decodes one complete bounded worker result after supervision succeeds.
    /// This validates data only; it is NOT evidence that a worker was sandboxed.
    ///
    /// # Errors
    /// Rejects malformed/version-skewed, oversized, empty, or mismatched data.
    pub fn decode(bytes: &[u8], expected: InputFormat) -> Result<Self, ImportError> {
        if bytes.len() > MAX_EXTRACTION_BYTES {
            return Err(ImportError::LimitExceeded);
        }
        let wire: ExtractionWire =
            serde_json::from_slice(bytes).map_err(|_| ImportError::InvalidExtraction)?;
        if wire.version != EXTRACTION_VERSION || wire.format != expected {
            return Err(ImportError::InvalidExtraction);
        }
        if wire.page_count == 0 || wire.page_count > MAX_PAGES || wire.blocks.len() > MAX_BLOCKS {
            return Err(ImportError::LimitExceeded);
        }
        let mut count = 0;
        let mut prior_page = 1;
        let mut readable = false;
        for block in &wire.blocks {
            if block.page < prior_page || block.page > wire.page_count {
                return Err(ImportError::InvalidExtraction);
            }
            prior_page = block.page;
            let length = block.text.chars().count();
            count += length;
            if length > MAX_BLOCK_CHARACTERS || count > MAX_EXTRACTED_CHARACTERS {
                return Err(ImportError::LimitExceeded);
            }
            if block
                .text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            {
                return Err(ImportError::UnsupportedControl);
            }
            readable |= block.text.chars().any(|c| !c.is_whitespace());
        }
        if !readable {
            return Err(ImportError::NoReadableText);
        }
        Ok(Self {
            format: wire.format,
            page_count: wire.page_count,
            blocks: wire.blocks,
        })
    }

    #[must_use]
    pub fn blocks(&self) -> &[ExtractedBlock] {
        &self.blocks
    }

    #[must_use]
    pub const fn format(&self) -> InputFormat {
        self.format
    }

    #[must_use]
    pub const fn page_count(&self) -> u16 {
        self.page_count
    }
}

impl std::fmt::Debug for ValidatedExtraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedExtraction")
            .field("format", &self.format)
            .field("page_count", &self.page_count)
            .field("block_count", &self.blocks.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Experience,
    Education,
    Skills,
    Projects,
    Summary,
    Certifications,
    Languages,
}

/// Exact, versioned heading aliases only. Unknown languages/headings are kept
/// literally as custom sections; casing never implies a person's identity.
#[must_use]
pub fn section_kind(heading: &str) -> Option<SectionKind> {
    match heading
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_lowercase()
        .as_str()
    {
        "experience"
        | "work experience"
        | "professional experience"
        | "experiencia"
        | "expérience"
        | "berufserfahrung"
        | "工作经历" => Some(SectionKind::Experience),
        "education" | "educación" | "formation" | "ausbildung" | "教育经历" => {
            Some(SectionKind::Education)
        }
        "skills" | "technical skills" | "habilidades" | "compétences" | "kenntnisse" | "技能" => {
            Some(SectionKind::Skills)
        }
        "projects" | "proyectos" | "projets" | "projekte" | "项目" => Some(SectionKind::Projects),
        "summary" | "professional summary" | "profile" | "perfil" | "profil" | "个人简介" => {
            Some(SectionKind::Summary)
        }
        "certifications" | "certificados" | "zertifikate" | "证书" => {
            Some(SectionKind::Certifications)
        }
        "languages" | "idiomas" | "langues" | "sprachen" | "语言" => Some(SectionKind::Languages),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactField {
    FullName,
    Email,
    Phone,
    Location,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ProposedContent {
    Section {
        heading: String,
        kind: Option<SectionKind>,
    },
    Contact {
        field: ContactField,
        value: String,
    },
    Text {
        text: String,
        is_bullet: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewReason {
    RecognizedHeading,
    CustomHeading,
    ExplicitContactLabel,
    UnclassifiedText,
    ListHint,
    EmptyBlock,
    NeedsSplitting,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProposalItem {
    /// Index into the immutable original extraction, not a filename or offset
    /// supplied by an external worker. Every block has exactly one proposal.
    pub source_index: usize,
    pub section_index: Option<usize>,
    pub content: ProposedContent,
    pub reasons: Vec<ReviewReason>,
}

pub struct ImportProposal {
    source: ValidatedExtraction,
    items: Vec<ProposalItem>,
}

impl ImportProposal {
    #[must_use]
    pub fn map(source: ValidatedExtraction) -> Self {
        let mut section_index = None;
        let mut items = Vec::with_capacity(source.blocks.len());
        for (index, block) in source.blocks.iter().enumerate() {
            let text = block.text.trim();
            let kind = section_kind(text);
            let (content, mut reasons) =
                if !text.is_empty() && (block.kind == BlockKind::Heading || kind.is_some()) {
                    section_index = Some(index);
                    (
                        ProposedContent::Section {
                            heading: text.to_owned(),
                            kind,
                        },
                        vec![if kind.is_some() {
                            ReviewReason::RecognizedHeading
                        } else {
                            ReviewReason::CustomHeading
                        }],
                    )
                } else if let Some((field, value)) = section_index
                    .is_none()
                    .then(|| labeled_contact(text))
                    .flatten()
                {
                    (
                        ProposedContent::Contact {
                            field,
                            value: value.to_owned(),
                        },
                        vec![ReviewReason::ExplicitContactLabel],
                    )
                } else {
                    let is_bullet = block.kind == BlockKind::ListItem;
                    let value = if is_bullet {
                        text.strip_prefix("- ")
                            .or_else(|| text.strip_prefix("• "))
                            .unwrap_or(text)
                    } else {
                        text
                    };
                    (
                        ProposedContent::Text {
                            text: value.to_owned(),
                            is_bullet,
                        },
                        vec![if text.is_empty() {
                            ReviewReason::EmptyBlock
                        } else if is_bullet {
                            ReviewReason::ListHint
                        } else {
                            ReviewReason::UnclassifiedText
                        }],
                    )
                };
            let limits = DocumentLimits::default();
            let maximum = if matches!(
                content,
                ProposedContent::Text {
                    is_bullet: true,
                    ..
                }
            ) {
                limits.bullet_characters
            } else {
                limits.field_characters
            };
            if text.chars().count() > maximum {
                reasons.push(ReviewReason::NeedsSplitting);
            }
            items.push(ProposalItem {
                source_index: index,
                section_index,
                content,
                reasons,
            });
        }
        Self { source, items }
    }

    #[must_use]
    pub fn source(&self) -> &ValidatedExtraction {
        &self.source
    }

    #[must_use]
    pub fn items(&self) -> &[ProposalItem] {
        &self.items
    }
}

impl std::fmt::Debug for ImportProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportProposal")
            .field("mapping_version", &MAPPING_VERSION)
            .field("item_count", &self.items.len())
            .finish_non_exhaustive()
    }
}

fn labeled_contact(text: &str) -> Option<(ContactField, &str)> {
    let (label, value) = text.split_once(':')?;
    let field = match label.trim().to_lowercase().as_str() {
        "name" | "full name" | "nombre" | "nom" | "姓名" => ContactField::FullName,
        "email" | "e-mail" | "correo" | "courriel" | "邮箱" => ContactField::Email,
        "phone" | "telephone" | "teléfono" | "téléphone" | "telefon" | "电话" => {
            ContactField::Phone
        }
        "location" | "ubicación" | "localisation" | "wohnort" | "所在地" => {
            ContactField::Location
        }
        _ => return None,
    };
    let value = value.trim();
    // Multi-line/empty values remain unclassified, not partially scraped.
    (!value.is_empty() && !value.contains(['\r', '\n'])).then_some((field, value))
}
