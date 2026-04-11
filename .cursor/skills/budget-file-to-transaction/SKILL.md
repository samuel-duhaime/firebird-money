---
name: budget-file-to-transaction
description: >-
  Converts budget exports into Transaction JSON for this repo. Do not apply from
  context alone; only when the user explicitly requests this skill by name
  (budget-file-to-transaction) or says to use the budget-to-transaction workflow.
---

# Budget file → Transaction

**Invocation only** — Apply this document when the user explicitly requests this skill, not from context alone.

## Output

**File:** Write the final JSON to `server/data/transactions/outputs/transactions_{n}.json`, where `{n}` is the next unused positive integer (e.g. no matching files → `transactions_1.json`; if `transactions_1.json` exists → `transactions_2.json`, and so on). List `server/data/transactions/outputs` for existing `transactions_*.json` names before choosing `n`.

JSON objects with **only** these keys (serde names in `server/src/main.rs`):

- `Date` — string, `YYYY-MM-DD` when possible; note assumption if ambiguous.
- `Description` — string, one line, trimmed.
- `Amount` — number (not string). Default: **expenses positive**; if the file uses the opposite, flip consistently and say once.
- `Category` — string from source, or short inferred label; note if inferred.

Emit `[...]` or `{ "transactions": [...] }` to mirror the sample API.

## Steps

1. Detect format/delimiter; skip headers, totals, blank rows.
2. Map columns → four keys (aliases e.g. posted/booking → Date; payee/memo/merchant → Description; debit/credit/value → Amount; class/type → Category).
3. Strip currency/`$`/thousands separators; normalize dates; collapse spaces in Description.
4. Output pretty JSON as `{ "transactions": [...] }`; save it to `server/data/transactions/outputs/transactions_{n}.json` as above; mention how many rows skipped if any.

## Example

`01/15/2024,STARBUCKS,-12.34,Restaurant` →

```json
{
  "transactions": [
    {
      "Date": "2024-01-15",
      "Description": "STARBUCKS",
      "Amount": 12.34,
      "Category": "Restaurant"
    }
  ]
}
```

(If negative = debit in source, use absolute value as expense unless user says otherwise.)
