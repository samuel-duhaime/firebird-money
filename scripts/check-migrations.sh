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
    # With a git ref, a file it doesn't have is new whatever its date says, since sqlx still runs it.
    if [ "$version" -le "$last_checked_migration" ]; then
        if [ -z "$base_ref" ] || git cat-file -e "$base_ref:$file" 2>/dev/null; then
            continue
        fi
    fi
    grep -qE -- '-- data-loss-reviewed:[[:space:]]*[^[:space:]]' "$file" && continue

    # Lines are joined so a statement split over two still matches. Comments are scanned too: a false
    # alarm costs a rewording, while parsing SQL here could miss a real statement.
    if tr -s '[:space:]' ' ' <"$file" | grep -qiE '\b(TRUNCATE|DELETE FROM|DROP (TABLE|COLUMN|SCHEMA))\b'; then
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
