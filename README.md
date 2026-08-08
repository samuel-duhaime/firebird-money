# FireBird Money

**Family finance, made simple.** FireBird Money puts your whole financial life in one place, and lets AI handle the busywork - **open source**, from day one.

## Table of Contents

- [Install](#install)
- [Configuration](#configuration)
- [How to run](#how-to-run)
- [API](#api)
- [Data model](#data-model)
- [Tests](#tests)
- [Lint & Format](#lint--format)
- [License](#license)

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
4. **[Claude Code](https://claude.com/product/claude-code)**, logged in — `POST /transactions/import` shells out to a `claude` subprocess (running the `budget-file-to-transaction` skill) to turn an uploaded budget file into transactions. `claude` must be on your `PATH` and authenticated, or every import job will fail.
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

The API is JSON, backed by Postgres.

`/transactions`:

- `GET /transactions` — list transactions, optionally filtered with `?date=YYYY-MM-DD`, `?start_date=`/`?end_date=` (inclusive range), `?merchant=`, and/or `?search=` (case-insensitive match against merchant, category, or amount). Accepts `?order=` (`date` [default], `inverse_date`, `amount`, `inverse_amount`).
- `GET /transactions/{id}` — fetch a single transaction.
- `POST /transactions` — create a transaction (`date`, `merchant`, `amount`, `category_id`, `account`).
- `PATCH /transactions/{id}` — partially update a transaction (only the fields you send change).
- `DELETE /transactions/{id}` — delete a transaction.
- `GET /transactions/download` — download the same filtered/sorted transactions as `GET /transactions`, rendered as a file. Accepts the same query params plus `?format=` (`csv` or `xlsx`, required).
- `POST /transactions/import` — upload a budget file (multipart, field `file`, 10 MB max) to import as transactions. Kicks off an async job and returns `202 Accepted` with a `Location` header pointing at the job. Requires the `claude` CLI (see [Install](#install)).
- `GET /transactions/import/jobs/{id}` — poll an import job's status (`pending`, `running`, `succeeded`, `failed`) and, once terminal, its `created_count`/`failed_count`/`skipped_count`/`error_message`.

Every transaction response includes its joined category: `category_name_en`, `category_name_fr`, and `category_type` alongside `category_id`. `category_id` must reference an existing category (enforced by a foreign key).

`/categories`:

- `GET /categories` — list all categories.
- `GET /categories/{id}` — fetch a single category.
- `POST /categories` — create a category (`name_en`, `name_fr`, `type`, where `type` is `income`, `expense`, or `transfer`).
- `PATCH /categories/{id}` — partially update a category (only the fields you send change).
- `DELETE /categories/{id}` — delete a category.

`/households`:

- `GET /households/{id}` — fetch a single household.
- `POST /households` — create a new, empty household.
- `DELETE /households/{id}` — delete a household. Fails while it still has members (see `/household-members`).

A household has no fields of its own beyond `id`/`created_at` — it's a pure container. Who belongs to it, and with what role, lives in `/household-members`.

`/users`:

- `GET /users/{id}` — fetch a single user.
- `POST /users` — create a user (`email`, `google_id` optional). `status` starts at `pending`.
- `PATCH /users/{id}` — partially update a user (`email`, `google_id`, `status`, `first_name`, `last_name`, `avatar_url`; only the fields you send change). `status` must be `verified`, `pending`, or `suspended`.
- `DELETE /users/{id}` — delete a user. Fails while they still belong to a household (see `/household-members`).

A user is a standalone login identity — how they relate to a household is recorded separately, since the same person can belong to more than one.

`/household-members`:

- `GET /household-members` — list memberships, optionally filtered by `?household_id=` and/or `?user_id=`.
- `GET /household-members/{id}` — fetch a single membership.
- `POST /household-members` — connect a user to a household with a role (`household_id`, `user_id`, `type`, where `type` is `family_manager` or `family_member`). A user has exactly one role per household.
- `PATCH /household-members/{id}` — change a membership's role (`type`).
- `DELETE /household-members/{id}` — remove a membership.

## Data model

The currently implemented API exposes `Category`, `Transaction`, `Household`, `User`, and `HouseholdMember` (see [API](#api) above). The diagram below is the planned full data model — Account, Institution, Merchant, Tag, and Rule are still design-stage entities, not yet implemented, and `HouseholdMember` (the join between `User` and `Household`) isn't pictured yet either:
![API class diagram](docs/images/api-diagram.png)

## Tests

Server only — the client has no tests yet.

```bash
cd server
cargo test
```

Each test runs against its own throwaway Postgres database (auto-migrated, auto-dropped), so your real data is untouched.

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
