use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ContactDivider {
    #[default]
    Dot,
    Bar,
    Dash,
}

impl ContactDivider {
    #[must_use]
    pub const fn is_default(&self) -> bool {
        matches!(self, Self::Dot)
    }

    #[must_use]
    pub const fn separator(self) -> &'static str {
        match self {
            Self::Dot => "  •  ",
            Self::Bar => "  |  ",
            Self::Dash => "  -  ",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContactDetails;

    #[test]
    fn default_preserves_legacy_serialization() {
        let legacy = r#"{"fullName":"Example","email":"","phone":"","location":"","links":[]}"#;
        let contact: ContactDetails = serde_json::from_str(legacy).unwrap();
        assert_eq!(contact.divider, ContactDivider::Dot);
        assert_eq!(serde_json::to_string(&contact).unwrap(), legacy);
    }

    #[test]
    fn selected_divider_round_trips_and_unknown_values_fail_closed() {
        for divider in [ContactDivider::Bar, ContactDivider::Dash] {
            let contact = ContactDetails {
                divider,
                ..ContactDetails::default()
            };
            let encoded = serde_json::to_string(&contact).unwrap();
            assert_eq!(
                serde_json::from_str::<ContactDetails>(&encoded).unwrap(),
                contact
            );
        }
        assert!(serde_json::from_str::<ContactDivider>("\"unknown\"").is_err());
    }
}
