//! The starter set of categories a newly created household gets, seeded once at creation (see
//! `repository::seed_defaults`). Moved here from the old global `seed_categories` migration now
//! that categories are per-household rather than shared by the whole app.

pub struct DefaultCategory {
    pub name_en: &'static str,
    pub name_fr: &'static str,
    pub r#type: &'static str,
}

pub const DEFAULT_CATEGORIES: &[DefaultCategory] = &[
    DefaultCategory {
        name_en: "Other",
        name_fr: "Autre",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Gift",
        name_fr: "Cadeau",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Animals",
        name_fr: "Animaux",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Cash",
        name_fr: "Comptant",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Culture",
        name_fr: "Culture",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Business expense",
        name_fr: "Dépense d’entreprise",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Education",
        name_fr: "Éducation",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Groceries",
        name_fr: "Épicerie",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Leisure",
        name_fr: "Loisir",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Rent",
        name_fr: "Loyer",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Home",
        name_fr: "Maison",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Restaurant",
        name_fr: "Restaurant",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Health",
        name_fr: "Santé",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Sports",
        name_fr: "Sport",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Tech",
        name_fr: "Tech",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Transport",
        name_fr: "Transport",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Work",
        name_fr: "Travail",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Vacation",
        name_fr: "Vacance",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Clothing",
        name_fr: "Vêtement",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Furniture",
        name_fr: "Ameublement",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Daycare",
        name_fr: "Garderie",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Insurance",
        name_fr: "Assurance",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Subscriptions",
        name_fr: "Abonnements",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Donations",
        name_fr: "Dons",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Unknown",
        name_fr: "Inconnu",
        r#type: "expense",
    },
    DefaultCategory {
        name_en: "Business income",
        name_fr: "Revenu d’entreprise",
        r#type: "income",
    },
    DefaultCategory {
        name_en: "Salary",
        name_fr: "Salaire",
        r#type: "income",
    },
    DefaultCategory {
        name_en: "Government",
        name_fr: "Gouvernement",
        r#type: "income",
    },
    DefaultCategory {
        name_en: "TFSA",
        name_fr: "CELI",
        r#type: "transfer",
    },
    DefaultCategory {
        name_en: "RRSP",
        name_fr: "REER",
        r#type: "transfer",
    },
    DefaultCategory {
        name_en: "FHSA",
        name_fr: "CELIAPP",
        r#type: "transfer",
    },
    DefaultCategory {
        name_en: "Credit card",
        name_fr: "Carte de crédit",
        r#type: "transfer",
    },
    DefaultCategory {
        name_en: "Transfer",
        name_fr: "Transfert",
        r#type: "transfer",
    },
];
