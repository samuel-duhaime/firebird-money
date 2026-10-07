//! The common merchants every household sees by default — shared singleton rows (`household_id`
//! `NULL`), not per-household copies, so unlike `category_groups`/`tags`' `DEFAULT_*` lists this
//! isn't seeded at household creation. See `repository::seed_defaults`, called once at server
//! startup instead.

/// A common merchant's real name, plus alternate spellings worth matching on (typed search, and raw
/// bank-statement text) without changing the name actually shown anywhere. Keeps `name` correct
/// ("McDonald's", not "McDonalds") while still being findable however someone actually types it —
/// most people drop the apostrophe, abbreviate a long name, or type a suffix-free version of a
/// subscription brand ("YouTube" for "YouTube Premium"). Both `name` and every alias are candidates
/// in `repository::best_match`, so an alias can make a merchant match raw statement text that its
/// `name` alone wouldn't.
pub struct DefaultMerchant {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
}

const fn merchant(name: &'static str) -> DefaultMerchant {
    DefaultMerchant {
        name,
        aliases: &[],
    }
}

pub const DEFAULT_MERCHANTS: &[DefaultMerchant] = &[
    // Shopping / retail
    merchant("Amazon"),
    merchant("Walmart"),
    merchant("Costco"),
    merchant("Canadian Tire"),
    merchant("Dollarama"),
    merchant("Best Buy"),
    merchant("IKEA"),
    merchant("Winners"),
    merchant("HomeSense"),
    merchant("Marshalls"),
    DefaultMerchant {
        name: "The Home Depot",
        aliases: &["Home Depot"],
    },
    merchant("Sport Chek"),
    merchant("Simons"),
    DefaultMerchant {
        name: "Hudson's Bay",
        aliases: &["Hudsons Bay", "Hudson Bay", "The Bay"],
    },
    merchant("Target"),
    merchant("Staples"),
    merchant("Indigo"),
    merchant("PetSmart"),
    merchant("Mondou"),
    merchant("Apple"),
    merchant("Google"),
    merchant("Microsoft"),
    // Entertainment / subscriptions
    merchant("Steam"),
    merchant("Netflix"),
    merchant("Spotify"),
    DefaultMerchant {
        name: "Disney+",
        aliases: &["Disney Plus", "Disney"],
    },
    // Kept separate from "Amazon" — a Prime membership charge and a retail purchase are different
    // enough expenses that most households will want to tell them apart.
    merchant("Amazon Prime"),
    merchant("Crave"),
    // "YouTube Premium" is the real subscription name, but most people just say/type "YouTube".
    DefaultMerchant {
        name: "YouTube Premium",
        aliases: &["YouTube"],
    },
    merchant("Audible"),
    merchant("Hulu"),
    merchant("HBO Max"),
    DefaultMerchant {
        name: "Paramount+",
        aliases: &["Paramount Plus", "Paramount"],
    },
    DefaultMerchant {
        name: "Apple TV+",
        aliases: &["Apple TV"],
    },
    // Coffee / fast food / restaurants
    merchant("Starbucks"),
    merchant("Tim Hortons"),
    merchant("Second Cup"),
    DefaultMerchant {
        name: "McDonald's",
        aliases: &["McDonalds", "McDonald", "McDo"],
    },
    merchant("Burger King"),
    DefaultMerchant {
        name: "Wendy's",
        aliases: &["Wendys", "Wendy"],
    },
    merchant("KFC"),
    merchant("Popeyes"),
    merchant("Taco Bell"),
    merchant("Subway"),
    merchant("A&W"),
    merchant("Pizza Pizza"),
    DefaultMerchant {
        name: "Domino's",
        aliases: &["Dominos", "Domino"],
    },
    merchant("Pizza Hut"),
    merchant("Chipotle"),
    merchant("Five Guys"),
    merchant("Dairy Queen"),
    DefaultMerchant {
        name: "Harvey's",
        aliases: &["Harveys", "Harvey"],
    },
    DefaultMerchant {
        name: "St-Hubert",
        aliases: &["St Hubert", "Saint-Hubert", "Saint Hubert"],
    },
    DefaultMerchant {
        name: "Chick-fil-A",
        aliases: &["Chick fil A", "Chickfila"],
    },
    // Delivery / rideshare / transit
    merchant("Uber"),
    merchant("Uber Eats"),
    merchant("DoorDash"),
    merchant("Lyft"),
    merchant("STM"),
    merchant("GO Transit"),
    merchant("VIA Rail"),
    merchant("Bixi"),
    // Groceries
    merchant("Metro"),
    merchant("IGA"),
    merchant("Maxi"),
    merchant("Provigo"),
    merchant("Super C"),
    merchant("Loblaws"),
    merchant("No Frills"),
    merchant("FreshCo"),
    merchant("Food Basics"),
    merchant("Sobeys"),
    merchant("Safeway"),
    merchant("Whole Foods"),
    DefaultMerchant {
        name: "Trader Joe's",
        aliases: &["Trader Joes", "Trader Joe"],
    },
    merchant("Kroger"),
    merchant("Publix"),
    // Pharmacy / health
    merchant("Jean Coutu"),
    merchant("Pharmaprix"),
    merchant("Uniprix"),
    merchant("Familiprix"),
    merchant("Rexall"),
    merchant("London Drugs"),
    merchant("Walgreens"),
    merchant("CVS"),
    merchant("GoodLife Fitness"),
    merchant("Planet Fitness"),
    // Gas
    DefaultMerchant {
        name: "Petro-Canada",
        aliases: &["Petro Canada"],
    },
    merchant("Esso"),
    merchant("Shell"),
    DefaultMerchant {
        name: "Couche-Tard",
        aliases: &["Couche Tard"],
    },
    merchant("Ultramar"),
    merchant("Chevron"),
    merchant("Mobil"),
    merchant("BP"),
    // Telecom
    merchant("Rogers"),
    merchant("Bell"),
    merchant("Telus"),
    merchant("Videotron"),
    merchant("Fizz"),
    merchant("Fido"),
    merchant("Koodo"),
    merchant("Virgin Plus"),
    merchant("Freedom Mobile"),
    // Travel
    merchant("Air Canada"),
    merchant("WestJet"),
    merchant("Airbnb"),
    merchant("Expedia"),
    merchant("Booking.com"),
    merchant("Marriott"),
    merchant("Hilton"),
    // Insurance
    merchant("Intact"),
    merchant("Sun Life"),
    merchant("Manulife"),
    merchant("State Farm"),
    merchant("Geico"),
    merchant("Allstate"),
    // Payments
    merchant("PayPal"),
    merchant("Venmo"),
    merchant("Interac"),
    merchant("Square"),
    // Banks
    merchant("Desjardins"),
    merchant("RBC"),
    merchant("TD"),
    merchant("Scotiabank"),
    merchant("BMO"),
    merchant("National Bank"),
    merchant("CIBC"),
    merchant("Chase"),
    merchant("Bank of America"),
    merchant("Wells Fargo"),
    merchant("Citibank"),
    merchant("Capital One"),
];
