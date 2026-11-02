#!/usr/bin/env bash
# Saves the Docker database to backups/ and prints the file's path. Only a backup that Postgres can
# read back gets its final name, so a *.dump file in backups/ is always a complete one.
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077

name="backups/firebird-money-$(date +%Y%m%d-%H%M%S)"
trap 'rm -f "$name.tmp"' EXIT

docker compose exec -T db pg_dump -U firebird -d firebird-money --format=custom >"$name.tmp"
docker compose exec -T db pg_restore --list <"$name.tmp" >/dev/null

mv "$name.tmp" "$name.dump"
echo "$name.dump"
