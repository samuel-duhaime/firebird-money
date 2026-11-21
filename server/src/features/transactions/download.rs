//! Renders transactions as CSV or Excel (`.xlsx`) bytes for `GET /transactions/download`.

use std::collections::HashMap;

use rust_xlsxwriter::{Workbook, XlsxError};
use unic_langid::LanguageIdentifier;

use super::model::{SortOrder, Transaction, TransactionFilter};
use crate::features::members::model::Member;
use crate::shared::l10n::L10n;

/// Fluent ids of the header row, in column order.
const HEADER_IDS: [&str; 7] = [
    "download-header-date",
    "download-header-merchant",
    "download-header-category",
    "download-header-tags",
    "download-header-account",
    "download-header-owner",
    "download-header-amount",
];

/// The words a downloaded file is written in: the language the person is using the app in, so its
/// headers, category names, "Shared" and filename read like the page they downloaded it from.
pub struct Labels {
    french: bool,
    headers: [String; 7],
    shared: String,
    transactions: String,
    from: String,
    to: String,
    until: String,
    oldest_first: String,
    highest_amount: String,
    lowest_amount: String,
}

impl Labels {
    pub fn new(l10n: &L10n, locale: &LanguageIdentifier) -> Self {
        let text = |id: &str| l10n.format_message(locale, id, None);
        Self {
            french: locale.language == "fr",
            headers: HEADER_IDS.map(text),
            shared: text("download-owner-shared"),
            transactions: text("download-filename-transactions"),
            from: text("download-filename-from"),
            to: text("download-filename-to"),
            until: text("download-filename-until"),
            oldest_first: text("download-filename-oldest-first"),
            highest_amount: text("download-filename-highest-amount"),
            lowest_amount: text("download-filename-lowest-amount"),
        }
    }

    /// The category's name in this language, as the app shows it.
    fn category<'a>(&self, transaction: &'a Transaction) -> &'a str {
        if self.french {
            &transaction.category_name_fr
        } else {
            &transaction.category_name_en
        }
    }
}

/// What the Owner column says for each member: their full name, else their email, as the app
/// shows them.
pub struct OwnerNames(HashMap<i32, String>);

impl OwnerNames {
    pub fn new(members: Vec<Member>) -> Self {
        Self(
            members
                .into_iter()
                .map(|member| {
                    let name = [member.first_name, member.last_name]
                        .into_iter()
                        .flatten()
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let name = if name.is_empty() { member.email } else { name };
                    (member.id, name)
                })
                .collect(),
        )
    }

    /// "Shared" (in `labels`' language) for no owner. An owner no longer in the household can't
    /// happen (removing a member makes their transactions Shared), but is left blank rather than
    /// failing the download.
    fn cell(&self, owner_user_id: Option<i32>, labels: &Labels) -> String {
        match owner_user_id {
            None => labels.shared.clone(),
            Some(id) => self.0.get(&id).cloned().unwrap_or_default(),
        }
    }
}

/// Comma-joins a transaction's tag names for a single spreadsheet cell, e.g. "Tax, Reimburse".
fn tags_cell(transaction: &Transaction) -> String {
    transaction
        .tags
        .iter()
        .map(|tag| tag.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Builds a filename like `transactions_highest-amount.csv` from the filters that were actually
/// applied, so the downloaded file's name reflects what's inside it.
pub fn filename(filter: &TransactionFilter, extension: &str, labels: &Labels) -> String {
    let mut parts = vec![labels.transactions.clone()];

    if let Some(date) = filter.date {
        parts.push(date.to_string());
    }
    match (filter.start_date, filter.end_date) {
        (Some(start), Some(end)) => parts.push(format!("{start}_{}_{end}", labels.to)),
        (Some(start), None) => parts.push(format!("{}_{start}", labels.from)),
        (None, Some(end)) => parts.push(format!("{}_{end}", labels.until)),
        (None, None) => {}
    }
    if let Some(search) = &filter.search {
        parts.push(slugify(search));
    }
    match filter.order {
        Some(SortOrder::InverseDate) => parts.push(labels.oldest_first.clone()),
        Some(SortOrder::Amount) => parts.push(labels.highest_amount.clone()),
        Some(SortOrder::InverseAmount) => parts.push(labels.lowest_amount.clone()),
        Some(SortOrder::Date) | None => {}
    }

    format!("{}.{extension}", parts.join("_"))
}

/// Lowercases `text` and replaces runs of non-alphanumeric characters with a single `-`, so it's
/// safe to embed in a filename. Truncated to keep filenames reasonable for long search terms.
fn slugify(text: &str) -> String {
    let dashed: String = text
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    dashed
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect()
}

/// Prefixes `value` with a `'` if it starts with `=`, `+`, `-`, or `@`, which spreadsheet apps
/// (Excel, Google Sheets) treat as the start of a formula. Without this, a user-controlled field
/// like merchant or category name (e.g. `=SUM(A1:A2)`) would execute as a formula when the CSV is
/// opened, rather than being displayed as plain text (CSV/formula injection).
fn escape_formula(value: &str) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@') => format!("'{value}"),
        _ => value.to_string(),
    }
}

/// Renders transactions as CSV bytes, with a header row.
pub fn to_csv(
    transactions: &[Transaction],
    owners: &OwnerNames,
    labels: &Labels,
) -> Result<Vec<u8>, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());

    writer.write_record(&labels.headers)?;
    for transaction in transactions {
        writer.write_record([
            transaction.date.to_string(),
            escape_formula(&transaction.merchant_name),
            escape_formula(labels.category(transaction)),
            escape_formula(&tags_cell(transaction)),
            escape_formula(&transaction.account_name),
            escape_formula(&owners.cell(transaction.owner_user_id, labels)),
            transaction.amount.to_string(),
        ])?;
    }

    writer.into_inner().map_err(|e| e.into_error().into())
}

/// Renders transactions as `.xlsx` bytes, with a header row.
pub fn to_xlsx(
    transactions: &[Transaction],
    owners: &OwnerNames,
    labels: &Labels,
) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (col, header) in labels.headers.iter().enumerate() {
        worksheet.write(0, col as u16, header)?;
    }
    for (index, transaction) in transactions.iter().enumerate() {
        let row = index as u32 + 1;
        worksheet.write(row, 0, transaction.date.to_string())?;
        worksheet.write(row, 1, &transaction.merchant_name)?;
        worksheet.write(row, 2, labels.category(transaction))?;
        worksheet.write(row, 3, tags_cell(transaction))?;
        worksheet.write(row, 4, &transaction.account_name)?;
        worksheet.write(row, 5, owners.cell(transaction.owner_user_id, labels))?;
        worksheet.write(row, 6, transaction.amount)?;
    }

    workbook.save_to_buffer()
}
