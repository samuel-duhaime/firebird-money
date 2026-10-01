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
    // TODO: name this after the household's actual first member once members have real display
    // names wired up here, instead of a placeholder.
    DefaultTag {
        name: "Member #1",
        color: "#9B51E0",
    },
];
