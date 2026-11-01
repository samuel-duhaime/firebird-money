#!/usr/bin/env bash
# Replaces the Docker database with a backup: ./scripts/restore.sh backups/<file>.dump
# Backs up the current database first, so a restore can itself be undone.
set -euo pipefail
cd "$(dirname "$0")/.."

backup="${1:-}"
if [ ! -f "$backup" ]; then
    echo "Usage: ./scripts/restore.sh backups/<file>.dump" >&2
    echo "Available backups:" >&2
    find backups -name '*.dump' | sort >&2
    exit 1
fi

read -r -p "Replace ALL current data with $backup? Type yes to continue: " answer
[ "$answer" = "yes" ] || { echo "Nothing changed."; exit 1; }

docker compose up -d --wait db

if safety="$(./scripts/backup.sh)"; then
    echo "Current data saved to $safety"
else
    read -r -p "The current data could not be backed up. Replace it anyway? Type yes to continue: " answer
    [ "$answer" = "yes" ] || { echo "Nothing changed."; exit 1; }
fi

# Nothing else may be writing to the database while it's replaced.
docker compose stop client server backup

docker compose exec -T db dropdb -U firebird --force --if-exists firebird-money
docker compose exec -T db createdb -U firebird firebird-money
docker compose exec -T db pg_restore -U firebird -d firebird-money \
    --no-owner --no-privileges --exit-on-error --single-transaction <"$backup"

docker compose up -d --wait --wait-timeout 180
echo "Restored $backup"
