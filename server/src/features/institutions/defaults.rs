//! The common institutions every household sees by default — shared singleton rows (`household_id`
//! `NULL`), seeded once at server startup by `repository::seed_defaults`, same as the common
//! merchants. Mostly Canadian, since that's who FireBird Money is for.

/// A common institution's name, its website (shown in the "Add account" picker), and other names
/// it's searchable by — e.g. its French name, or the short form people actually type.
pub struct DefaultInstitution {
    pub name: &'static str,
    pub website_url: &'static str,
    pub aliases: &'static [&'static str],
}

const fn institution(
    name: &'static str,
    website_url: &'static str,
    aliases: &'static [&'static str],
) -> DefaultInstitution {
    DefaultInstitution {
        name,
        website_url,
        aliases,
    }
}

pub const DEFAULT_INSTITUTIONS: &[DefaultInstitution] = &[
    // Big banks and credit unions
    institution(
        "Desjardins",
        "https://www.desjardins.com",
        &["Caisse Desjardins"],
    ),
    institution(
        "RBC Royal Bank",
        "https://www.rbcroyalbank.com",
        &["RBC", "Royal Bank", "Banque Royale"],
    ),
    institution("TD Canada Trust", "https://www.td.com", &["TD"]),
    institution(
        "BMO",
        "https://www.bmo.com",
        &["Bank of Montreal", "Banque de Montréal"],
    ),
    institution(
        "Scotiabank",
        "https://www.scotiabank.com",
        &["Banque Scotia"],
    ),
    institution("CIBC", "https://www.cibc.com", &[]),
    institution(
        "National Bank",
        "https://www.nbc.ca",
        &["Banque Nationale", "BNC", "NBC"],
    ),
    institution(
        "Laurentian Bank",
        "https://www.laurentianbank.ca",
        &["Banque Laurentienne"],
    ),
    institution("ATB Financial", "https://www.atb.com", &["ATB"]),
    // Online banks
    institution("Tangerine", "https://www.tangerine.ca", &[]),
    institution("EQ Bank", "https://www.eqbank.ca", &["EQ"]),
    institution("Simplii Financial", "https://www.simplii.com", &["Simplii"]),
    institution(
        "PC Financial",
        "https://www.pcfinancial.ca",
        &["PC Money", "President's Choice"],
    ),
    institution("KOHO", "https://www.koho.ca", &[]),
    institution("Neo Financial", "https://www.neofinancial.com", &["Neo"]),
    institution(
        "Manulife Bank",
        "https://www.manulifebank.ca",
        &["Manulife"],
    ),
    // Credit cards
    institution(
        "American Express",
        "https://www.americanexpress.com/ca",
        &["Amex"],
    ),
    institution(
        "Canadian Tire Bank",
        "https://www.ctfs.com",
        &["Triangle", "Canadian Tire"],
    ),
    // Brokerages
    institution("Wealthsimple", "https://www.wealthsimple.com", &[]),
    institution("Questrade", "https://www.questrade.com", &[]),
    institution(
        "Disnat",
        "https://www.disnat.com",
        &["Desjardins Online Brokerage"],
    ),
];
