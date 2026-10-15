//! The starter set of tags a newly created household gets, seeded once at creation — see
//! `repository::seed_defaults`. Based on Monarch's default tags.

pub struct DefaultTag {
    pub name: &'static str,
    pub color: &'static str,
}

pub const DEFAULT_TAGS: &[DefaultTag] = &[
    DefaultTag {
        name: "Tax",
        color: "#2F80ED",
    },
    DefaultTag {
        name: "Reimburse",
        color: "#56CCF2",
    },
    DefaultTag {
        name: "Split",
        color: "#27AE60",
    },
    DefaultTag {
        name: "Business",
        color: "#F2994A",
    },
    DefaultTag {
        name: "Subscription",
        color: "#F2C94C",
    },
];

/// The placeholder member tag households used to be seeded with, before they got one named after
/// each member — see `repository::name_existing_member_tag`.
pub const PLACEHOLDER_MEMBER_TAG_NAME: &str = "Member #1";

/// Color of the extra starter tag named after the household's first member (see `member_tag_name`).
pub const MEMBER_TAG_COLOR: &str = "#9B51E0";

/// The starter tag name for a household member: their first name plus their last name's initial,
/// e.g. "John S." for John Smith — or just "John" without a last name.
pub fn member_tag_name(first_name: &str, last_name: Option<&str>) -> String {
    match last_name.and_then(|name| name.chars().next()) {
        Some(initial) => format!("{first_name} {}.", initial.to_uppercase()),
        None => first_name.to_string(),
    }
}
