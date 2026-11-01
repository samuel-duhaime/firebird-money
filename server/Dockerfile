# Builds the API in a full Rust image, then copies only the compiled binary into a small one.
FROM rust:1.93-bookworm AS build
WORKDIR /app

# Compile the dependencies on their own first, so a code change doesn't rebuild them all.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src

COPY src src
COPY migrations migrations
COPY locales locales
# Migrations are compiled into the binary. COPY keeps each file's old date, so without this Cargo
# could think the placeholder build above is newer and ship a server missing the latest ones.
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
# ca-certificates lets the server make HTTPS calls (Claude, Resend).
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home firebird
COPY --from=build /app/target/release/server /usr/local/bin/server
USER firebird
EXPOSE 3055
CMD ["server"]
