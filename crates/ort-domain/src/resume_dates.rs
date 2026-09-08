use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, ValidationError};

/// Precision is explicit: an absent month is a year, never an inferred January.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarDate {
    pub year: u16,
    pub month: Option<u8>,
    pub expected: bool,
}

impl CalendarDate {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        if !(1..=9999).contains(&self.year) || self.month.is_some_and(|m| !(1..=12).contains(&m)) {
            return Err(ValidationError::InvalidDate);
        }
        Ok(())
    }

    #[must_use]
    pub fn display_text(&self) -> String {
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let value = self
            .month
            .and_then(|m| m.checked_sub(1))
            .and_then(|m| MONTHS.get(usize::from(m)))
            .map_or_else(
                || self.year.to_string(),
                |month| format!("{month} {}", self.year),
            );
        if self.expected {
            format!("Expected {value}")
        } else {
            value
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DateEnd {
    Present,
    Date { value: CalendarDate },
}

/// One stable, ordered date or range. Either endpoint may be omitted. The label
/// can describe graduation, certification, or another optional date detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeDate {
    pub id: EntityId,
    pub order: u16,
    pub label: String,
    pub start: Option<CalendarDate>,
    pub end: Option<DateEnd>,
}

impl ResumeDate {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        if let Some(start) = &self.start {
            start.validate()?;
        }
        if let Some(DateEnd::Date { value }) = &self.end {
            value.validate()?;
        }
        Ok(())
    }

    /// A warning, not a persistence rejection. Mixed precision is reversed only
    /// when the known bounds cannot overlap; no missing month is invented.
    #[must_use]
    pub fn is_reversed(&self) -> bool {
        match (&self.start, &self.end) {
            (Some(start), Some(DateEnd::Date { value: end })) => {
                (start.year, start.month.unwrap_or(1)) > (end.year, end.month.unwrap_or(12))
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn display_text(&self) -> String {
        let start = self.start.as_ref().map(CalendarDate::display_text);
        let end = self.end.as_ref().map(|end| match end {
            DateEnd::Present => "Present".to_owned(),
            DateEnd::Date { value } => value.display_text(),
        });
        let value = match (start, end) {
            (Some(start), Some(end)) => format!("{start}–{end}"),
            (Some(value), None) | (None, Some(value)) => value,
            (None, None) => return String::new(),
        };
        if self.label.trim().is_empty() {
            value
        } else {
            format!("{}: {value}", self.label.trim())
        }
    }
}
