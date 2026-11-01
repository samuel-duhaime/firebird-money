#!/usr/bin/env bash
# Guards real data against migrations, which run by themselves on every update. Fails when:
# - a new migration deletes data, unless it says why that's safe with a line like
#   "-- data-loss-reviewed: the rows are copied to accounts above"
# - an existing migration is edited or removed, given a git ref to compare with (the server refuses
#   to start on a database that already ran a different version of it).
set -euo pipefail
cd "$(dirname "$0")/.."

# Migrations up to this one predate real data, and some of them do wipe tables.
last_checked_migration="20261019010000"
base_ref="${1:-}"
failed=0

for file in server/migrations/*.sql; do
    version="$(basename "$file" | cut -d_ -f1)"
    [ "$version" -gt "$last_checked_migration" ] || continue
    grep -q -- '-- data-loss-reviewed:' "$file" && continue

    # Comments are stripped so that mentioning "truncate" in a sentence doesn't count.
    if sed 's/--.*$//' "$file" | grep -qiE '\b(TRUNCATE|DELETE[[:space:]]+FROM|DROP[[:space:]]+(TABLE|COLUMN))\b'; then
        echo "$file deletes data. Keep the data instead, or explain with a '-- data-loss-reviewed:' line." >&2
        failed=1
    fi
done

if [ -n "$base_ref" ]; then
    changed="$(git diff --name-only --diff-filter=MDR "$base_ref"...HEAD -- server/migrations)"
    if [ -n "$changed" ]; then
        echo "Existing migrations were edited or removed. Add a new migration instead:" >&2
        echo "$changed" >&2
        failed=1
    fi
fi

exit "$failed"
