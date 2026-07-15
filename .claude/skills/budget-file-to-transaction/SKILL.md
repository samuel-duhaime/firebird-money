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
{ "date": "2024-01-15", "merchant": "STARBUCKS", "amount": "12.34", "category_id": 12, "account": "User 1" }
```

- `date` — `YYYY-MM-DD`.
- `merchant` — payee, one line, trimmed.
- `amount` — always positive, no currency symbol. Expenses and income (deposits, paycheck, e-transfers) are both positive; direction is shown by category, not sign.
- `category_id` — id of the best-matching row from `GET http://127.0.0.1:3055/categories` (see Steps). Use the `Unknown` category's id if nothing fits.
- `account` — `User 1` unless the source says otherwise.

## Steps

1. `GET /categories` once; keep the returned `id`/`name_en`/`name_fr`/`type` list for matching.
2. Parse the source: detect format, skip headers/totals/blanks, map columns, normalize dates/amounts/merchant.
3. For each row, match its best-guess category name against the fetched list (by `name_en` or `name_fr`); fall back to `Unknown` if nothing fits.
4. Preview every row (date, merchant, amount, category name, account) plus skip count; wait for confirmation.
5. On confirmation, `POST` each row; track successes/failures.
6. Report created/failed/skipped counts.

## Example

`01/15/2024,STARBUCKS,-12.34,Restaurant` → preview row `2024-01-15 | STARBUCKS | 12.34 | Restaurant | User 1` → matched `Restaurant` to id `12` → on confirmation:

```bash
curl -X POST http://127.0.0.1:3055/transactions \
  -H "Content-Type: application/json" \
  -d '{"date":"2024-01-15","merchant":"STARBUCKS","amount":"12.34","category_id":12,"account":"User 1"}'
```


