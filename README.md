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

## How to run

```bash
cd server
cargo run
```

Open **http://127.0.0.1:3055/transactions/sample** for the same CSV as `server/data/transactions/outputs/transactions_sample.csv` (header row, then one transaction per line). Converted exports go under `server/data/transactions/outputs/` as `transactions_{n}.csv`; open **http://127.0.0.1:3055/transactions/1** (replace `1` with `n`) to download that file. Missing files return 404; read failures return 500 with a short message.
