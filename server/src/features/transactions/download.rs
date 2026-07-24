//! Renders transactions as CSV or Excel (`.xlsx`) bytes for `GET /transactions/download`.

use rust_xlsxwriter::{Workbook, XlsxError};

use super::model::{SortOrder, Transaction, TransactionFilter};

const HEADERS: [&str; 4] = ["Date", "Merchant", "Category", "Amount"];

/// Builds a filename like `transactions_starbucks_highest-amount.csv` from the filters that were
/// actually applied, so the downloaded file's name reflects what's inside it.
pub fn filename(filter: &TransactionFilter, extension: &str) -> String {
    let mut parts = vec!["transactions".to_string()];

    if let Some(date) = filter.date {
        parts.push(date.to_string());
    }
    if let Some(merchant) = &filter.merchant {
        parts.push(slugify(merchant));
    }
    if let Some(search) = &filter.search {
        parts.push(slugify(search));
    }
    match filter.order {
        Some(SortOrder::InverseDate) => parts.push("oldest-first".to_string()),
        Some(SortOrder::Amount) => parts.push("highest-amount".to_string()),
        Some(SortOrder::InverseAmount) => parts.push("lowest-amount".to_string()),
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

/// Renders transactions as CSV bytes, with a header row.
pub fn to_csv(transactions: &[Transaction]) -> Result<Vec<u8>, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());

    writer.write_record(HEADERS)?;
    for transaction in transactions {
        writer.write_record([
            transaction.date.to_string(),
            transaction.merchant.clone(),
            transaction.category_name_en.clone(),
            transaction.amount.to_string(),
        ])?;
    }

    writer.into_inner().map_err(|e| e.into_error().into())
}

/// Renders transactions as `.xlsx` bytes, with a header row.
pub fn to_xlsx(transactions: &[Transaction]) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (col, header) in HEADERS.iter().enumerate() {
        worksheet.write(0, col as u16, *header)?;
    }
    for (index, transaction) in transactions.iter().enumerate() {
        let row = index as u32 + 1;
        worksheet.write(row, 0, transaction.date.to_string())?;
        worksheet.write(row, 1, &transaction.merchant)?;
        worksheet.write(row, 2, &transaction.category_name_en)?;
        worksheet.write(row, 3, transaction.amount)?;
    }

    workbook.save_to_buffer()
}
