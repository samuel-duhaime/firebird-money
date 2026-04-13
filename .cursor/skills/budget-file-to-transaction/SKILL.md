---
name: budget-file-to-transaction
description: >-
  Converts budget exports into transaction CSV for this repo. Do not apply from
  context alone; only when the user explicitly requests this skill by name
  (budget-file-to-transaction) or says to use the budget-to-transaction workflow.
---

# Budget file → Transaction CSV

**Invocation only** — Apply this document when the user explicitly requests this skill, not from context alone.

## Output

**File:** Write the final CSV to `server/data/transactions/outputs/transactions_{n}.csv`, where `{n}` is the next unused positive integer (e.g. no matching files → `transactions_1.csv`; if `transactions_1.csv` exists → `transactions_2.csv`, and so on). List `server/data/transactions/outputs` for existing `transactions_*.csv` names before choosing `n`.

UTF-8 CSV with **only** these columns (header row required, same order as the sample):

- `Date` — string, `YYYY-MM-DD` when possible; note assumption if ambiguous.
- `Description` — string, one line, trimmed; use standard CSV quoting if the text contains comma, quote, or newline.
- `Amount` — number (plain decimal in the cell, no currency symbol). Default: **expenses positive**; if the file uses the opposite, flip consistently and say once.
- `Category` — string from source, or short inferred label; note if inferred.

Match the shape of `server/data/transactions/outputs/transactions_sample.csv` (header + one data row per transaction).

## Steps

1. Detect format/delimiter; skip headers, totals, blank rows.
2. Map columns → the four columns above (aliases e.g. posted/booking → Date; payee/memo/merchant → Description; debit/credit/value → Amount; class/type → Category).
3. Strip currency/`$`/thousands separators; normalize dates; collapse spaces in Description.
4. Write UTF-8 CSV with one header line then one line per transaction; save to `server/data/transactions/outputs/transactions_{n}.csv` as above; mention how many rows skipped if any.

## Example

`01/15/2024,STARBUCKS,-12.34,Restaurant` → CSV row:

```csv
Date,Description,Amount,Category
2024-01-15,STARBUCKS,12.34,Restaurant
```

(If negative = debit in source, use absolute value as expense unless user says otherwise.)
