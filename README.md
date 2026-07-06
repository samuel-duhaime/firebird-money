# Budget Robot

Right now this is a small script (and API) to help with **my family budget**. The goal is to make that workflow **as easy as possible**.

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

`DATABASE_URL` must point at a reachable Postgres database, e.g. `postgres://user:password@localhost:5432/budget_robot`. The schema is created automatically: every `cargo run` applies any pending migrations from `server/migrations/` on startup.

## How to run

```bash
cd server
cargo run
```

The API is JSON, backed by Postgres, under `/transactions`:

- `GET /transactions` — list transactions, optionally filtered with `?date=YYYY-MM-DD` and/or `?merchant=`.
- `GET /transactions/{id}` — fetch a single transaction.
- `POST /transactions` — create a transaction (`date`, `merchant`, `amount`, `category_id`, `account`).
- `PATCH /transactions/{id}` — partially update a transaction (only the fields you send change).
- `DELETE /transactions/{id}` — delete a transaction.

## Tests

```bash
cd server
cargo test
```

Each test runs against its own throwaway Postgres database (auto-migrated, auto-dropped), so your real data is untouched.
