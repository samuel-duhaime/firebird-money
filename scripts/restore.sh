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

# Nothing else may be writing to the database from here on, so the safety backup misses nothing.
docker compose stop client server backup

# However this ends from here, the app comes back up and says how it went.
outcome="The restore did not finish, see the messages above. Your current data is unchanged."
trap 'docker compose up -d --wait --wait-timeout 180; echo "$outcome"' EXIT

if safety="$(./scripts/backup.sh)"; then
    echo "Current data saved to $safety"
else
    read -r -p "The current data could not be backed up. Replace it anyway? Type yes to continue: " answer
    if [ "$answer" != "yes" ]; then
        outcome="Nothing changed."
        exit 1
    fi
fi

# Restored beside the current database, so a damaged backup can't leave you with nothing. The swap
# then happens in one transaction: it either fully happens or doesn't happen at all.
scratch="firebird-money-restore"
previous="firebird-money-previous"
docker compose exec -T db dropdb -U firebird --force --if-exists "$scratch"
docker compose exec -T db dropdb -U firebird --force --if-exists "$previous"
docker compose exec -T db createdb -U firebird "$scratch"
if ! docker compose exec -T db pg_restore -U firebird -d "$scratch" \
    --no-owner --no-privileges --exit-on-error --single-transaction <"$backup"; then
    docker compose exec -T db dropdb -U firebird --force "$scratch"
    outcome="$backup could not be restored. Nothing changed."
    exit 1
fi

docker compose exec -T db psql -U firebird -d postgres -v ON_ERROR_STOP=1 --single-transaction \
    -c "ALTER DATABASE \"firebird-money\" RENAME TO \"$previous\"" \
    -c "ALTER DATABASE \"$scratch\" RENAME TO \"firebird-money\""

outcome="Restored $backup"
docker compose exec -T db dropdb -U firebird --force "$previous"
