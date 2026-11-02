#!/bin/sh
# Runs inside the `backup` container. Once an hour, makes a backup if the newest one is over a day
# old, so a computer that's often off still gets one. Deletes backups older than 30 days, but only
# after a new one succeeds, so a failing backup never eats the good ones.
# The first backup of each month is also copied to /backups/monthly, and kept for good. Files under
# 10 kB are skipped: those are of a database with no tables yet, made during the very first start.
umask 077
mkdir -p /backups/monthly

while true; do
    if [ -z "$(find /backups -maxdepth 1 -name '*.dump' -mmin -1440)" ]; then
        name="/backups/firebird-money-$(date +%Y%m%d-%H%M%S)"
        if pg_dump --format=custom --file="$name.tmp" && pg_restore --list "$name.tmp" >/dev/null; then
            mv "$name.tmp" "$name.dump"
            find /backups -maxdepth 1 -name '*.dump' -mtime +30 -delete
            echo "Backup saved: $name.dump"
        else
            rm -f "$name.tmp"
            echo "BACKUP FAILED, will try again in an hour" >&2
        fi
    fi

    monthly="/backups/monthly/firebird-money-$(date +%Y-%m).dump"
    first_this_month="$(find /backups -maxdepth 1 -name "firebird-money-$(date +%Y%m)*.dump" -size +10k | sort | head -n 1)"
    if [ ! -f "$monthly" ] && [ -n "$first_this_month" ]; then
        if cp "$first_this_month" "$monthly.tmp" && mv "$monthly.tmp" "$monthly"; then
            echo "Monthly backup saved: $monthly"
        else
            rm -f "$monthly.tmp"
            echo "MONTHLY BACKUP FAILED, will try again in an hour" >&2
        fi
    fi

    sleep 3600
done
