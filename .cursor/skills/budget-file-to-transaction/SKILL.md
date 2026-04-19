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

UTF-8 CSV. Use headers in the **language the user asked for**:

- English: `Date,Merchant,Amount,Category,Account`
- French: `Date,Marchand,Montant,Catégorie,Compte`

Columns:

- Date — `YYYY-MM-DD` when possible.
- Merchant / Marchand — merchant/payee (one line, trimmed).
- Amount / Montant — number (no currency). Default: **expenses positive**; if the source uses the opposite, flip consistently and say once.
- Category / Catégorie — use locale labels (see below). If unsure: Other / Autre.
- Account / Compte — for now, set to `User 1` for all rows unless the source provides it.

Match the shape of `server/data/transactions/outputs/transactions_sample.csv` (header + one data row per transaction).

## Steps

1. Detect format/delimiter; skip headers, totals, blank rows.
2. Map source columns → the output columns.
3. Strip currency/`$`/thousands separators; normalize dates; collapse spaces in Merchant/Marchand.
4. Write UTF-8 CSV with one header line then one line per transaction; save to `server/data/transactions/outputs/transactions_{n}.csv` as above; mention how many rows skipped if any.

## Example

`01/15/2024,STARBUCKS,-12.34,Restaurant` → CSV row:

```csv
Date,Merchant,Amount,Category,Account
2024-01-15,STARBUCKS,12.34,Restaurant,User 1
```

(If negative = debit in source, use absolute value as expense unless user says otherwise.)

## Categories / Catégories

- French: use labels from `server/locales/fr.ftl` (e.g. `Épicerie`, `Restaurant`, `Transport`, `Santé`, …). Fallback: `Autre`.
- English: use the corresponding English labels from `server/locales/en.ftl`. Fallback: `Other`.
