# Budget Robot

Right now this is a small script (and API) to help with **my family budget**. The goal is to make that workflow **as easy as possible**.

## Install

1. **Rust** — Install the stable toolchain with [rustup](https://rustup.rs/) so you have `cargo` on your PATH.
2. **This repo** — Clone it (or download it), then from the repo root:

```bash
cd server
cargo build
```

The first `cargo build` (or `cargo run`) downloads dependencies and compiles the server.

## Configuration

Copy the example environment file into place, then edit it if you need to:

```bash
cd server
cp .env.example .env
```

## How to run

```bash
cd server
cargo run
```

Open **http://127.0.0.1:3055/transactions** to list transactions from the database (optionally filtered with `?date=` and/or `?merchant=`), or **http://127.0.0.1:3055/transactions/1** (replace `1` with an id) to fetch a single one. `POST /transactions` creates a transaction, `PATCH /transactions/{id}` updates one, and `DELETE /transactions/{id}` removes one.
