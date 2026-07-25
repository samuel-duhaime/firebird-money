---
name: budget-file-to-transaction
description: >-
  Converts a budget export into transactions and inserts them into the local
  database via the server's API. Unattended — invoked only by the server's
  `POST /transactions/import` background job, never interactively. Do not
  apply from context alone; only when the invocation explicitly names this
  skill and supplies a `job_id`.
---

# Budget file → Transaction

**Unattended only.** This runs as a server-spawned subprocess with no human present to confirm
anything and no session persistence — there is exactly one chance to get it right, and exactly one
chance to report the result back. Only `Read` and the three `curl` prefixes below are pre-approved;
anything else is denied automatically, so stick to the calls shown here exactly.

The invocation gives a file path and a `job_id`. If either is missing, stop and report `failed`
via the final `PATCH` (see Steps) with an `error_message` explaining what's missing — do not guess.

## Output

`POST http://127.0.0.1:3055/transactions` per transaction (server must be running).

```json
{ "date": "2024-01-15", "merchant": "STARBUCKS", "amount": "12.34", "category_id": 12, "account": "User 1", "reviewed": false }
```

- `date` — `YYYY-MM-DD`.
- `merchant` — payee, one line, trimmed.
- `amount` — always positive, no currency symbol. Expenses and income (deposits, paycheck, e-transfers) are both positive; direction is shown by category, not sign.
- `category_id` — id of the best-matching row from `GET http://127.0.0.1:3055/categories` (see Steps). Use the `Unknown` category's id if nothing fits.
- `account` — `User 1` unless the source says otherwise.
- `reviewed` — always `false`. No human confirmed these rows; this flags them for later review.

## Steps

1. Fetch categories once, using exactly this command (matches the pre-approved allowlist —
   don't add flags like `-s`, they can cause the call to be denied):
   ```bash
   curl http://127.0.0.1:3055/categories
   ```
   Keep the returned `id`/`name_en`/`name_fr`/`type` list for matching.
2. Read and parse the source file: detect format, skip headers/totals/blanks, map columns, normalize dates/amounts/merchant.
3. For each row, match its best-guess category name against the fetched list (by `name_en` or `name_fr`); fall back to `Unknown` if nothing fits.
4. `POST` each row with `"reviewed": false`; track successes/failures/skips. No preview, no confirmation step — go straight from matching to posting.
5. Report the result with exactly one final call, matching one of:
   ```bash
   curl -X PATCH http://127.0.0.1:3055/transactions/import/jobs/{job_id} \
     -H "Content-Type: application/json" \
     -d '{"status":"succeeded","created_count":N,"failed_count":M,"skipped_count":K}'
   ```
   ```bash
   curl -X PATCH http://127.0.0.1:3055/transactions/import/jobs/{job_id} \
     -H "Content-Type: application/json" \
     -d '{"status":"failed","error_message":"<what went wrong>"}'
   ```
   Use `failed` if the file couldn't be parsed at all or every row failed to post; `succeeded`
   otherwise, even if some rows were skipped or failed individually (their counts say so). If this
   call is skipped, the server falls back to marking the job failed on its own once the subprocess
   exits, using whatever it printed as the error message — so still worth getting this right.

## Example

`01/15/2024,STARBUCKS,-12.34,Restaurant` → matched `Restaurant` to id `12`:

```bash
curl -X POST http://127.0.0.1:3055/transactions \
  -H "Content-Type: application/json" \
  -d '{"date":"2024-01-15","merchant":"STARBUCKS","amount":"12.34","category_id":12,"account":"User 1","reviewed":false}'
```
