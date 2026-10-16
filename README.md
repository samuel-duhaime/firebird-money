# FireBird Money

**Family finance, made simple.** FireBird Money puts your whole financial life in one place, and lets AI handle the busywork - **open source**, from day one.

## Table of Contents

- [Screenshots](#screenshots)
- [Install](#install)
- [Configuration](#configuration)
- [How to run](#how-to-run)
- [API](#api)
- [Data schema](#data-schema)
- [Tests](#tests)
- [Lint & Format](#lint--format)
- [License](#license)

## Screenshots

![Transactions page](docs/images/transactions-page.png)

## Install

### Server

1. **Rust** — Install the stable toolchain with [rustup](https://rustup.rs/) so you have `cargo` on your PATH.
2. **PostgreSQL** — Install it and have a server running locally (e.g. `sudo apt install postgresql`). Then create a database for this project (any name, matched to `DATABASE_URL` below), e.g.:

   ```bash
   psql -h localhost -U postgres -W -c 'CREATE DATABASE "firebird-money";'
   ```

   You'll be prompted for the `postgres` user's password — use that same password in `DATABASE_URL` in `.env` below.

   Migrations create the tables automatically on startup, but they don't create the database itself — skipping this step fails with `database "firebird-money" does not exist`.
3. **cargo-watch** (used by the VS Code "Run Server" task for auto-reload on save) — `cargo install cargo-watch`.
4. **A [Claude API](https://platform.claude.com) key** (optional) — statement import (`POST /transactions/import/read`) reads uploaded bank statements with Claude. Set `ANTHROPIC_API_KEY` in `.env` below; without it the server still runs, and statement import answers `503`.
5. **This repo** — Clone it (or download it), then from the repo root:

   ```bash
   cd server
   cargo build
   ```

   The first `cargo build` (or `cargo run`) downloads dependencies and compiles the server.

### Client

1. **Node.js** — Install a recent LTS version (includes `npm`).
2. **This repo** — Clone it (or download it), then from the repo root:

   ```bash
   cd client
   npm install
   ```

## Configuration

### Server

Copy the example environment file into place, then edit it with your own Postgres credentials:

```bash
cd server
cp .env.example .env
```

`DATABASE_URL` must point at a reachable Postgres database, e.g. `postgres://user:password@localhost:5432/firebird-money`. The schema is created automatically: every `cargo run` applies any pending migrations from `server/migrations/` on startup.

`DEFAULT_LANGUAGE` sets the server's response locale — `"en"` or `"fr"`. Unset or invalid falls back to English.

Sign-in is passwordless (see [`/auth`](#api)). Out of the box, `SKIP_EMAIL_VERIFICATION="true"` skips the email and signs you in right away, so localhost needs no mail provider.

To send real emails instead — which works from localhost too, since it's just an outbound HTTPS call — set `RESEND_API_KEY` to a [Resend](https://resend.com) key and `SKIP_EMAIL_VERIFICATION="false"`. The default `EMAIL_FROM`, `onboarding@resend.dev`, only delivers to the email address on your Resend account; sending to anyone else needs a verified domain and an `EMAIL_FROM` on it.

The server refuses to start if the shortcut is on while `APP_ENV="production"`, if a real send has no API key, or if `APP_ENV` isn't exactly `"production"` or `"development"`. See `.env.example` for every variable and its accepted values.

### Client

Copy the example environment file into place:

```bash
cd client
cp .env.example .env
```

`VITE_API_BASE_URL` must point at the running server, e.g. `http://localhost:3055`.

## How to run

### Server

```bash
cd server
cargo watch -x run # Or `cargo run` for a single run without auto-reload.
```

Or use the VS Code "Run Server" task for auto-reload on save.

### Client

```bash
cd client
npm run dev
```

Or use the VS Code "Run Client" task.

### Both at once

In VS Code, run the "Run Client and Server" task (`Ctrl+Shift+P` → `Tasks: Run Task`) to start the server and client together, each with auto-reload on save.

## API

The API is JSON, backed by Postgres, and requires a session. `transactions`, `categories`,
`category-groups`, and `tags` are scoped to the caller's household. `merchants` is mostly scoped the
same way, except for a shared common set every household can see but never modify — see
[`/merchants`](#api).

`/auth`:

- `POST /auth/request-login` — email a one-time magic link (`email`, optional `language`). Creates the account if the address is new. Answers `{"status": "email_sent"}`, or `{"status": "signed_in", "session": {…}}` when `SKIP_EMAIL_VERIFICATION` is on.
- `GET /auth/verify?token=` — spend the token and open a session. Returns the `GET /auth/me` payload; `400` if the token is unknown, expired, or already spent.
- `GET /auth/me` — the signed-in `user` and the `household` they belong to (`null` before onboarding). `pending_onboarding_steps` lists the onboarding steps (`name`, `household`) the user still has to finish — the client keeps sending them back to `/onboarding` until it's empty. Finished steps are stored in `user.onboarding_steps`. `401` without a session.
- `POST /auth/logout` — end the session. Idempotent.
- `POST /auth/onboarding` — save the caller's name (`first_name` required, `last_name` optional) and create a household (`family_manager`) or, with a `join_code`, join one (`family_member`), then record both steps in `onboarding_steps`. A caller who already has a household only has their name saved. `400` without a first name, `404` for an unknown code, `409` if already onboarded or already a member.

Magic links last 15 minutes and work once. The session cookie is httpOnly and SameSite=Lax, Secure in production, so browser calls need `fetch(…, { credentials: 'include' })`.

`/transactions`:

- `GET /transactions` — list the caller's household's transactions, optionally filtered with `?date=YYYY-MM-DD`, `?start_date=`/`?end_date=` (inclusive range), `?merchant_id=`, `?account_id=`, and/or `?search=` (case-insensitive match against merchant name, original statement, category, or amount). Accepts `?order=` (`date` [default], `inverse_date`, `amount`, `inverse_amount`).
- `GET /transactions/{id}` — fetch a single transaction.
- `POST /transactions` — create a transaction (`date`, `amount`, `category_id`, `account_id`, optional `tag_ids`, optional `reviewed` [defaults to `true`], and either `merchant_id` and/or `original_statement`). `household_id` and `user_id` are never read from the body — they're always the caller's own household and user.
- `PATCH /transactions/{id}` — partially update a transaction (only the fields you send change). `tag_ids`, if given, *replaces* the full tag set (`[]` clears it). There's no `original_statement` here — it's immutable after creation; only `merchant_id` is editable.
- `DELETE /transactions/{id}` — delete a transaction.
- `PATCH /transactions/bulk` — partially update several transactions at once (`ids`, `patch` — `patch` supports `date`, `merchant_id`, `category_id`, `account_id`, and `tag_ids`). Unlike the single-transaction `PATCH`, `tag_ids` here *adds* to each transaction's existing tags rather than replacing them. Ids the household doesn't own are silently skipped.
- `DELETE /transactions/bulk` — delete several transactions at once (`ids`). Ids the household doesn't own are silently skipped.
- `GET /transactions/download` — download the same filtered/sorted transactions as `GET /transactions`, rendered as a file (including Tags and Account columns). Accepts the same query params plus `?format=` (`csv` or `xlsx`, required).
- `POST /transactions/import/read` — read a bank statement (multipart: `file`, 10 MB max — a PDF, a PNG/JPG/GIF/WebP image, CSV/text, or an Excel/OpenDocument spreadsheet, converted to CSV text in memory first; optional `account_id`, the account it's from) with Claude. Nothing is stored: the file is only held in memory for the one Claude call, and Claude only sees it plus the household's account, category, tag, and active merchant names. Streams newline-delimited JSON progress events (`reading`, `found` with a running `count`, `matching`, `checking_duplicates`), ending with `done` and the lines to review (`date`, `original_statement`, `amount`, `merchant_id` *or* `new_merchant_name`, `category_id`, `category_confident`, `account_id`, `tag_ids`, `duplicate_of_transaction_id`) or `error`. Requires `ANTHROPIC_API_KEY` (`503` otherwise).
- `POST /transactions/import` — add the statement lines the user accepted (`transactions`: each with `date`, `original_statement`, `amount`, `category_id`, `account_id`, optional `tag_ids`, and exactly one of `merchant_id`/`new_merchant_name`). All or nothing; each is stored with `source: "import"`, and any new merchant is created. Returns `201` with `created_count` and `created_merchant_count`.

Every transaction response includes its joined category (`category_name_en`, `category_name_fr`, `category_type`), its joined merchant (`merchant_name`, `merchant_logo_url`), its account (`account_id`, `account_name`), where it came from (`source`: `manual` or `import`), the immutable `original_statement` it was created from, and its attached tags (`tags`, each `{id, name, color}`). `category_id` and `account_id` must belong to the caller's own household (`400` otherwise); any `tag_ids` outside it are silently dropped rather than rejected.

On create, `merchant_id` and `original_statement` are both optional but at least one is required: given `merchant_id` alone, `original_statement` defaults to that merchant's own name (the manual "Add Transaction" shape); given `original_statement` alone, the server matches an existing merchant by case-insensitive substring (checking common merchants and the household's own, including each one's aliases) or creates a new custom merchant named exactly the given text (see [`/merchants`](#api)). A `merchant_id` that doesn't belong to the caller's household (and isn't common) is rejected with `400`, same as an unknown `category_id`.

`/merchants`:

- `GET /merchants` — list merchants visible to the caller's household (the shared common set plus this household's own), each with how many of the household's transactions currently carry it and, if any exist, a `recommended_category_id` — the category most often used with that merchant in this household's own transaction history (ties broken by whichever was used most recently), `null` until there's at least one. Accepts `?order=` (`transaction_count` [default] or `alphabetical`).
- `GET /merchants/{id}` — fetch a single merchant.
- `POST /merchants` — create a custom merchant for the caller's household (`name`).
- `PATCH /merchants/{id}` — rename a custom merchant. Common merchants (shared across every household) can't be changed this way — same as any id outside the caller's household, it 404s.
- `DELETE /merchants/{id}` — delete a custom merchant. Fails while it's still used by existing transactions; common merchants can't be deleted at all.

A merchant with `household_id: null` is common — seeded once at server startup (not per household, unlike categories/tags), shared by every household, and immune to the household-scoped update/delete above. Everything else belongs to exactly one household. A common merchant may also carry `aliases` (alternate spellings it also matches on, e.g. "McDonalds" for "McDonald's") — curated server-side, not settable through this API.

`/institutions`:

- `GET /institutions` — list institutions visible to the caller's household (the shared common set plus this household's own), A-Z, each with how many of the household's accounts are held there (`account_count`).
- `GET /institutions/{id}` — fetch a single institution.
- `POST /institutions` — create a custom institution for the caller's household (`name`, optional `website_url` — must start with `http://` or `https://`).
- `PATCH /institutions/{id}` — rename a custom institution or change its `website_url` (`""` clears it). Common institutions 404, same as merchants.
- `DELETE /institutions/{id}` — delete a custom institution. Fails while it still has accounts; common institutions can't be deleted at all.

An institution is the bank, credit union, or brokerage an account is held at. Like merchants, one with `household_id: null` is common — seeded once at server startup (Desjardins, RBC, TD, National Bank, Wealthsimple, …) and shared by every household, with `aliases` it's also searchable by (e.g. "Banque Nationale"). Everything else belongs to exactly one household.

`/accounts`:

- `GET /accounts` — list the caller's household's accounts, grouped by institution (A-Z), each with its `institution_name` and `transaction_count`.
- `GET /accounts/{id}` — fetch a single account.
- `POST /accounts` — create an account (`institution_id`, `name`, `type`, optional `last_four`, optional `currency` [defaults to `CAD`]). `type` is one of `checking`, `savings`, `credit_card`, `investment`, `loan`, `mortgage`, `cash`, or `other`; `last_four` is 1-4 letters or digits; `currency` is a 3-letter code. `institution_id` must be common or the household's own.
- `PATCH /accounts/{id}` — partially update an account (only the fields you send change; `last_four: ""` clears it).
- `DELETE /accounts/{id}` — delete an account. Fails while it's still used by existing transactions.

Every transaction is in exactly one account (`account_id` on [`/transactions`](#api)). No balances or bank sync yet.

`/categories`:

- `GET /categories` — list the caller's household's categories, each in its display order relative to the others in its group and with how many transactions currently carry it.
- `GET /categories/{id}` — fetch a single category.
- `POST /categories` — create a category (`group_id`, `name_en`, `name_fr`). `group_id` must reference a category group in the caller's own household. Appended at the end of its group's order.
- `PATCH /categories/{id}` — partially update a category (only the fields you send change).
- `PATCH /categories/reorder` — set the display order of the given categories (`category_ids`, every id in the new order). Pass just one group's ids to reorder within that group alone. Returns every category in its (possibly unaffected) order.
- `DELETE /categories/{id}` — delete a category. Fails while it's still used by existing transactions.

`type` lives on the category's group, not the category. Names are unique per household, not globally.

`/category-groups`:

- `GET /category-groups` — list the caller's household's category groups, in their display order.
- `GET /category-groups/{id}` — fetch a single category group.
- `POST /category-groups` — create a category group (`name_en`, `name_fr`, `type`, where `type` is `income`, `expense`, or `transfer`). Appended at the end of the household's order.
- `PATCH /category-groups/{id}` — partially update a category group (only the fields you send change).
- `PATCH /category-groups/reorder` — set the household's category group display order (`category_group_ids`, every id in the new order). Returns the groups in their new order.
- `DELETE /category-groups/{id}` — delete a category group. Fails while it still has categories in it.

New households are seeded with starter groups and categories automatically.

`/tags`:

- `GET /tags` — list the caller's household's tags, in their display order, with each tag's `transaction_count`.
- `GET /tags/{id}` — fetch a single tag.
- `POST /tags` — create a tag (`name`, `color`). Unlike categories, a tag has a single free-form `name` — no `name_en`/`name_fr` pair. Appended at the end of the household's order.
- `PATCH /tags/{id}` — partially update a tag (only the fields you send change).
- `PATCH /tags/reorder` — set the household's tag display order (`tag_ids`, every id in the new order). Returns the tags in their new order.
- `DELETE /tags/{id}` — delete a tag. Fails while it's still used by existing transactions.

A tag is a free-form label attached to transactions (see `tag_ids` on [`/transactions`](#api)), independent of category or account. Names are unique per household, not globally. New households are seeded with starter tags automatically.

`/households`:

- `GET /households/{id}` — fetch a single household.
- `POST /households` — create a new household, seeded with its starter category groups, categories, and tags.
- `DELETE /households/{id}` — delete a household. Fails while it still has data connected to it (members, category groups, categories, tags, custom merchants, custom institutions, accounts, or transactions).

Beyond `id`/`created_at`, a household carries only a `join_code`, generated on creation: the code an existing member shares so someone else can join through `POST /auth/onboarding`. Who belongs to it, and with what role, lives in `/members`.

`/users`:

- `GET /users/{id}` — fetch a single user.
- `POST /users` — create a user (`email`, `google_id` optional). `status` starts at `pending`.
- `PATCH /users/{id}` — partially update a user (`email`, `google_id`, `status`, `first_name`, `last_name`, `avatar_url`; only the fields you send change). `status` must be `verified`, `pending`, or `suspended`.
- `DELETE /users/{id}` — delete a user. Fails while they still belong to a household (see `/members`).

A user is a login identity, plus the (at most one) household they belong to and their role in it (`household_id`, `household_role`; both `null` until onboarding). In practice signing in creates them (see [`/auth`](#api)); these routes are for direct management.

`/members`:

- `GET /members` — list the users in the caller's household, each with their `role`, `email`, `first_name`, `last_name`, and `status`. A member's `id` is their user id.
- `GET /members/{id}` — fetch a single member.
- `PATCH /members/{id}` — change a member's `role` (`family_manager` or `family_member`). Managers only.
- `DELETE /members/{id}` — remove a member from the household (their user account is kept). Managers only. Fails while they still have transactions in it.

Nobody is added here: people join a household through `POST /auth/onboarding` with its `join_code`.

`/settings`:

- `GET /settings` — the signed-in user's display preferences (`show_category_column`, `show_tags_column`, `show_account_column`), defaulted to every column visible if they've never saved any.
- `PATCH /settings` — partially update the signed-in user's display preferences (only the fields you send change). Creates the row on the user's first write.

Settings are per-user, not per-household: which optional columns show on the transactions list.

## Data schema

Every table in the Postgres schema (see `server/migrations/`) and how they relate. `Rule` is still design-stage — not a real table yet — so it isn't pictured.

**People & access:**

```mermaid
erDiagram
    HOUSEHOLDS ||--o{ USERS : has
    USERS ||--o{ LOGIN_TOKENS : has
    USERS ||--o{ SESSIONS : has
    USERS ||--o| SETTINGS : has
```

**Household finances:**

```mermaid
erDiagram
    HOUSEHOLDS ||--o{ CATEGORY_GROUPS : has
    HOUSEHOLDS ||--o{ CATEGORIES : has
    CATEGORY_GROUPS ||--o{ CATEGORIES : has
    HOUSEHOLDS ||--o{ TRANSACTIONS : has
    CATEGORIES ||--o{ TRANSACTIONS : categorizes
    USERS ||--o{ TRANSACTIONS : creates
    MERCHANTS ||--o{ TRANSACTIONS : identifies
    HOUSEHOLDS ||--o{ MERCHANTS : "owns custom"
    HOUSEHOLDS ||--o{ INSTITUTIONS : "owns custom"
    HOUSEHOLDS ||--o{ ACCOUNTS : has
    INSTITUTIONS ||--o{ ACCOUNTS : holds
    ACCOUNTS ||--o{ TRANSACTIONS : contains
    HOUSEHOLDS ||--o{ TAGS : has
    TRANSACTIONS ||--o{ TRANSACTION_TAGS : has
    TAGS ||--o{ TRANSACTION_TAGS : has
```

## Tests

### Server

```bash
cd server
cargo test
```

Each test runs against its own throwaway Postgres database (auto-migrated, auto-dropped), so your real data is untouched.

### Client (Playwright, end-to-end)

```bash
cd client
npx playwright install   # first time only
npm run test:ui
```

Each worker gets its own throwaway server, Vite instance, and Postgres database — same idea as the server tests above, just per Playwright worker instead of per test. Override `PLAYWRIGHT_PG_ADMIN_URL` if your local Postgres differs from `postgres://postgres:postgres@localhost:5432/postgres`.

To regenerate the [Screenshots](#screenshots) images:

```bash
cd client
npm run screenshot:readme
```

## Lint & Format

### Server

```bash
cd server
cargo fmt
```

### Client

```bash
cd client
npm run lint    # oxlint
npm run format  # prettier
```

### Both at once

In VS Code, run the "Format" task (`Ctrl+Shift+P` → `Tasks: Run Task`) to format both server and client at once.

## License

MIT — see [LICENSE](LICENSE).
