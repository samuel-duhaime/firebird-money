//! The starter set of category groups (and the categories inside each) a newly created household
//! gets, seeded once at creation — see `repository::seed_defaults`. Based on Monarch's default
//! category structure, plus a household's registered Canadian savings accounts.

use crate::features::categories::defaults::DefaultCategory;

pub struct DefaultCategoryGroup {
    pub name_en: &'static str,
    pub name_fr: &'static str,
    pub r#type: &'static str,
    pub categories: &'static [DefaultCategory],
}

pub const DEFAULT_CATEGORY_GROUPS: &[DefaultCategoryGroup] = &[
    DefaultCategoryGroup {
        name_en: "Income",
        name_fr: "Revenus",
        r#type: "income",
        categories: &[
            DefaultCategory {
                name_en: "Paychecks",
                name_fr: "Chèques de paie",
            },
            DefaultCategory {
                name_en: "Interest",
                name_fr: "Intérêts",
            },
            DefaultCategory {
                name_en: "Business Income",
                name_fr: "Revenu d’entreprise",
            },
            DefaultCategory {
                name_en: "Other Income",
                name_fr: "Autre revenu",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Gifts & Donations",
        name_fr: "Cadeaux et dons",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Charity",
                name_fr: "Bienfaisance",
            },
            DefaultCategory {
                name_en: "Gifts",
                name_fr: "Cadeaux",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Auto & Transport",
        name_fr: "Auto et transport",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Auto Payment",
                name_fr: "Paiement auto",
            },
            DefaultCategory {
                name_en: "Public Transit",
                name_fr: "Transport en commun",
            },
            DefaultCategory {
                name_en: "Gas",
                name_fr: "Essence",
            },
            DefaultCategory {
                name_en: "Auto Maintenance",
                name_fr: "Entretien auto",
            },
            DefaultCategory {
                name_en: "Parking & Tolls",
                name_fr: "Stationnement et péages",
            },
            DefaultCategory {
                name_en: "Taxi & Ride Shares",
                name_fr: "Taxi et covoiturage",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Housing",
        name_fr: "Logement",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Mortgage",
                name_fr: "Hypothèque",
            },
            DefaultCategory {
                name_en: "Rent",
                name_fr: "Loyer",
            },
            DefaultCategory {
                name_en: "Home Improvement",
                name_fr: "Rénovations",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Bills & Utilities",
        name_fr: "Factures et services",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Garbage",
                name_fr: "Ordures",
            },
            DefaultCategory {
                name_en: "Water",
                name_fr: "Eau",
            },
            DefaultCategory {
                name_en: "Gas & Electric",
                name_fr: "Gaz et électricité",
            },
            DefaultCategory {
                name_en: "Internet & Cable",
                name_fr: "Internet et câble",
            },
            DefaultCategory {
                name_en: "Phone",
                name_fr: "Téléphone",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Food & Dining",
        name_fr: "Alimentation",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Groceries",
                name_fr: "Épicerie",
            },
            DefaultCategory {
                name_en: "Restaurants & Bars",
                name_fr: "Restaurants et bars",
            },
            DefaultCategory {
                name_en: "Coffee Shops",
                name_fr: "Cafés",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Travel & Lifestyle",
        name_fr: "Voyage et style de vie",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Travel & Vacation",
                name_fr: "Voyage et vacances",
            },
            DefaultCategory {
                name_en: "Entertainment & Recreation",
                name_fr: "Divertissement et loisirs",
            },
            DefaultCategory {
                name_en: "Personal",
                name_fr: "Personnel",
            },
            DefaultCategory {
                name_en: "Pets",
                name_fr: "Animaux",
            },
            DefaultCategory {
                name_en: "Fun Money",
                name_fr: "Argent plaisir",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Shopping",
        name_fr: "Magasinage",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Shopping",
                name_fr: "Magasinage",
            },
            DefaultCategory {
                name_en: "Clothing",
                name_fr: "Vêtements",
            },
            DefaultCategory {
                name_en: "Furniture & Housewares",
                name_fr: "Meubles et articles ménagers",
            },
            DefaultCategory {
                name_en: "Electronics",
                name_fr: "Électronique",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Children",
        name_fr: "Enfants",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Child Care",
                name_fr: "Garderie",
            },
            DefaultCategory {
                name_en: "Child Activities",
                name_fr: "Activités pour enfants",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Education",
        name_fr: "Éducation",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Student Loans",
                name_fr: "Prêts étudiants",
            },
            DefaultCategory {
                name_en: "Education",
                name_fr: "Éducation",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Health & Wellness",
        name_fr: "Santé et bien-être",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Medical",
                name_fr: "Médical",
            },
            DefaultCategory {
                name_en: "Dentist",
                name_fr: "Dentiste",
            },
            DefaultCategory {
                name_en: "Fitness",
                name_fr: "Conditionnement physique",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Financial",
        name_fr: "Finances",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Loan Repayment",
                name_fr: "Remboursement de prêt",
            },
            DefaultCategory {
                name_en: "Financial & Legal Services",
                name_fr: "Services financiers et juridiques",
            },
            DefaultCategory {
                name_en: "Financial Fees",
                name_fr: "Frais financiers",
            },
            DefaultCategory {
                name_en: "Cash & ATM",
                name_fr: "Comptant et guichet automatique",
            },
            DefaultCategory {
                name_en: "Insurance",
                name_fr: "Assurance",
            },
            DefaultCategory {
                name_en: "Taxes",
                name_fr: "Impôts",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Other",
        name_fr: "Autre",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Uncategorized",
                name_fr: "Non catégorisé",
            },
            DefaultCategory {
                name_en: "Check",
                name_fr: "Chèque",
            },
            DefaultCategory {
                name_en: "Miscellaneous",
                name_fr: "Divers",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Business",
        name_fr: "Entreprise",
        r#type: "expense",
        categories: &[
            DefaultCategory {
                name_en: "Advertising & Promotion",
                name_fr: "Publicité et promotion",
            },
            DefaultCategory {
                name_en: "Business Utilities & Communication",
                name_fr: "Services publics et communications d’entreprise",
            },
            DefaultCategory {
                name_en: "Employee Wages & Contract Labor",
                name_fr: "Salaires et main-d’œuvre contractuelle",
            },
            DefaultCategory {
                name_en: "Business Travel & Meals",
                name_fr: "Voyages et repas d’affaires",
            },
            DefaultCategory {
                name_en: "Business Auto Expenses",
                name_fr: "Frais d’auto d’entreprise",
            },
            DefaultCategory {
                name_en: "Business Insurance",
                name_fr: "Assurance d’entreprise",
            },
            DefaultCategory {
                name_en: "Office Supplies & Expenses",
                name_fr: "Fournitures et dépenses de bureau",
            },
            DefaultCategory {
                name_en: "Office Rent",
                name_fr: "Loyer de bureau",
            },
            DefaultCategory {
                name_en: "Postage & Shipping",
                name_fr: "Affranchissement et expédition",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Transfers",
        name_fr: "Transferts",
        r#type: "transfer",
        categories: &[
            DefaultCategory {
                name_en: "Transfer",
                name_fr: "Transfert",
            },
            DefaultCategory {
                name_en: "Credit Card Payment",
                name_fr: "Paiement de carte de crédit",
            },
            DefaultCategory {
                name_en: "Balance Adjustments",
                name_fr: "Ajustements de solde",
            },
        ],
    },
    DefaultCategoryGroup {
        name_en: "Savings & Investments",
        name_fr: "Épargne et placements",
        r#type: "transfer",
        categories: &[
            DefaultCategory {
                name_en: "TFSA",
                name_fr: "CELI",
            },
            DefaultCategory {
                name_en: "RRSP",
                name_fr: "REER",
            },
            DefaultCategory {
                name_en: "FHSA",
                name_fr: "CELIAPP",
            },
            DefaultCategory {
                name_en: "RESP",
                name_fr: "REEE",
            },
            DefaultCategory {
                name_en: "Non-Registered",
                name_fr: "Non enregistré",
            },
        ],
    },
];
