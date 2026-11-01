#!/usr/bin/env bash
# Starts the Docker app on the code in this folder, or updates it after a `git pull`. Each step only
# runs if the one before it worked, so a failed build or backup leaves the running app untouched.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Building the new version"
docker compose build

echo "==> Backing up the database"
docker compose up -d --wait db
backup="$(./scripts/backup.sh)"
echo "Saved $backup"

echo "==> Starting the new version"
if ! docker compose up -d --wait --wait-timeout 180; then
    echo >&2
    echo "The new version did not start. Your data is still in the database, and any migration" >&2
    echo "that failed was undone. See what went wrong with:" >&2
    echo "  docker compose logs server" >&2
    echo "To go back: check out the previous version with git, then run:" >&2
    echo "  ./scripts/restore.sh $backup" >&2
    echo "  ./scripts/update.sh" >&2
    exit 1
fi

echo "==> Done: http://localhost:$(docker compose port client 80 | cut -d: -f2)"
