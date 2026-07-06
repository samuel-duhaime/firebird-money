---
name: budget-file-to-transaction
description: >-
  Converts budget exports into transactions and inserts them into the local
  database via the server's API. Do not apply from context alone; only when
  the user explicitly requests this skill by name (budget-file-to-transaction)
  or says to use the budget-to-transaction workflow.
---

# Budget file → Transaction

**Invocation only** — apply only when explicitly requested by name.

## Output

`POST http://127.0.0.1:3055/transactions` per transaction (server must be running via `cargo run` in `server/`).

```json
{ "date": "2024-01-15", "merchant": "STARBUCKS", "amount": "12.34", "category_id": 1, "account": "User 1" }
```

- `date` — `YYYY-MM-DD`.
- `merchant` — payee, one line, trimmed.
- `amount` — always positive, no currency symbol. Expenses and income (deposits, paycheck, e-transfers) are both positive; direction is shown by category, not sign.
- `category_id` — always `1` (no `categories` table yet). Still pick a best-guess category label for the preview.
- `account` — `User 1` unless the source says otherwise.

## Steps

1. Parse the source: detect format, skip headers/totals/blanks, map columns, normalize dates/amounts/merchant.
2. Preview every row (date, merchant, amount, category, account) plus skip count; wait for confirmation.
3. On confirmation, `POST` each row; track successes/failures.
4. Report created/failed/skipped counts.

## Example

`01/15/2024,STARBUCKS,-12.34,Restaurant` → preview row `2024-01-15 | STARBUCKS | 12.34 | Restaurant | User 1` → on confirmation:

```bash
curl -X POST http://127.0.0.1:3055/transactions \
  -H "Content-Type: application/json" \
  -d '{"date":"2024-01-15","merchant":"STARBUCKS","amount":"12.34","category_id":1,"account":"User 1"}'
```


