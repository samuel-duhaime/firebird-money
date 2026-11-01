# FireBird Money

## Client (`client/`)

- Use arrow functions for components and helpers, not `function` declarations (e.g. `const Dashboard = () => {}`, not `function Dashboard() {}`).

## Migrations (`server/migrations/`)

Migrations run by themselves on every update, against real data, so `scripts/check-migrations.sh` (in CI) enforces these:

- Never delete data (`TRUNCATE`, `DELETE FROM`, `DROP TABLE`, `DROP COLUMN`) without first moving what's worth keeping. If it's truly safe, say why in a `-- data-loss-reviewed: ...` line.
- Never edit or remove a migration that's already merged: the server refuses to start on a database that ran a different version. Add a new one.
