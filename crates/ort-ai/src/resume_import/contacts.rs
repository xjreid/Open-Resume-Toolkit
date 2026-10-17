//! Personal contact sections belong to the single top contact block. Preserve
//! unclassified contact text as additional editable contact lines.
use super::{Contact, ContactLink, Proposal, contact_url};

fn add_link(contact: &mut Contact, label: String, url: String) {
    if !contact.links.iter().any(|link| {
        link.label == label && contact_url(&link.url, 0).is_ok_and(|existing| existing == url)
    }) {
        contact.links.push(ContactLink { label, url });
    }
}

fn add_text(contact: &mut Contact, text: &str) {
    if [
        contact.full_name.as_str(),
        contact.email.as_str(),
        contact.phone.as_str(),
    ]
    .contains(&text)
        || contact.location.lines().any(|line| line == text)
    {
        return;
    }
    if let Some(rest) = text.strip_prefix('[')
        && let Some((label, address)) = rest.split_once("](")
        && let Some(address) = address.strip_suffix(')')
        && !label.contains(']')
        && let Ok(url) = contact_url(address, 0)
    {
        add_link(contact, label.into(), url);
    } else {
        if !contact.location.is_empty() {
            contact.location.push('\n');
        }
        contact.location.push_str(text);
    }
}

pub(super) fn consolidate(proposal: &mut Proposal) {
    let mut sections = Vec::new();
    for section in std::mem::take(&mut proposal.sections) {
        let heading = section.heading.trim().trim_end_matches(':').to_lowercase();
        if !matches!(
            heading.as_str(),
            "contact"
                | "contact information"
                | "contact details"
                | "personal information"
                | "social links"
                | "social profiles"
        ) {
            sections.push(section);
            continue;
        }
        for entry in section.entries {
            let mut values: Vec<_> = [
                entry.title,
                entry.details,
                entry.role,
                entry.date,
                entry.location,
                entry.extra,
            ]
            .into_iter()
            .chain(entry.main_info.items)
            .filter(|value| !value.trim().is_empty())
            .collect();
            // A common model response is an entry titled "GitHub" with a URL
            // in its next slot. Retain that exact label/destination together.
            if values.len() == 2
                && let Ok(url) = contact_url(&values[1], 0)
            {
                add_link(&mut proposal.contact, values.remove(0), url);
            } else {
                for value in values {
                    add_text(&mut proposal.contact, &value);
                }
            }
        }
    }
    proposal.sections = sections;
}
