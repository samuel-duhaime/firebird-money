# FireBird Money

**Family finance, made simple.** FireBird Money puts your whole financial life in one place, and lets AI handle the busywork - **open source**, from day one.

## Table of Contents

- [Install](#install)
- [Configuration](#configuration)
- [How to run](#how-to-run)
- [API](#api)
- [Tests](#tests)

## Install

1. **Rust** — Install the stable toolchain with [rustup](https://rustup.rs/) so you have `cargo` on your PATH.
2. **PostgreSQL** — Install it and have a server running locally (e.g. `sudo apt install postgresql`. Create a database for this project (any name, matched to `DATABASE_URL` below).
3. **This repo** — Clone it (or download it), then from the repo root:

```bash
cd server
cargo build
```

The first `cargo build` (or `cargo run`) downloads dependencies and compiles the server.

## Configuration

Copy the example environment file into place, then edit it with your own Postgres credentials:

```bash
cd server
cp .env.example .env
```

`DATABASE_URL` must point at a reachable Postgres database, e.g. `postgres://user:password@localhost:5432/firebird-money`. The schema is created automatically: every `cargo run` applies any pending migrations from `server/migrations/` on startup.

## How to run

```bash
cd server
cargo run
```

## API

The API is JSON, backed by Postgres.

`/transactions`:

- `GET /transactions` — list transactions, optionally filtered with `?date=YYYY-MM-DD` and/or `?merchant=`.
- `GET /transactions/{id}` — fetch a single transaction.
- `POST /transactions` — create a transaction (`date`, `merchant`, `amount`, `category_id`, `account`).
- `PATCH /transactions/{id}` — partially update a transaction (only the fields you send change).
- `DELETE /transactions/{id}` — delete a transaction.

Every transaction response includes its joined category: `category_name_en`, `category_name_fr`, and `category_type` alongside `category_id`. `category_id` must reference an existing category (enforced by a foreign key).

`/categories`:

- `GET /categories` — list all categories.
- `GET /categories/{id}` — fetch a single category.
- `POST /categories` — create a category (`name_en`, `name_fr`, `type`, where `type` is `income`, `expense`, or `transfer`).
- `PATCH /categories/{id}` — partially update a category (only the fields you send change).
- `DELETE /categories/{id}` — delete a category.

## Tests

```bash
cd server
cargo test
```

Each test runs against its own throwaway Postgres database (auto-migrated, auto-dropped), so your real data is untouched.
