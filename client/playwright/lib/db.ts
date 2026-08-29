import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client, Pool } from 'pg';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '../../..');

/**
 * Falls back to the server's own local `DATABASE_URL` (`server/.env`) pointed at the `postgres`
 * maintenance database instead of the app's — that file is already set up with working local
 * credentials per the README's Install section, so a fresh clone doesn't need
 * `PLAYWRIGHT_PG_ADMIN_URL` configured separately just to match them.
 */
const adminUrlFromServerEnv = (): string | undefined => {
  const envPath = path.join(repoRoot, 'server/.env');
  if (!fs.existsSync(envPath)) return undefined;

  const match = fs
    .readFileSync(envPath, 'utf8')
    .match(/^DATABASE_URL="?([^"\n]+)"?/m);
  if (!match) return undefined;

  const url = new URL(match[1]);
  url.pathname = '/postgres';
  return url.toString();
};

/**
 * Admin connection used to create/drop each worker's throwaway database — points at Postgres's
 * built-in `postgres` maintenance database, never at a worker database itself (you can't
 * `DROP DATABASE` the one you're connected to, and `CREATE DATABASE` can't run inside a
 * transaction block). The final fallback matches CI's `postgres:16` service exactly.
 */
const ADMIN_URL =
  process.env.PLAYWRIGHT_PG_ADMIN_URL ??
  adminUrlFromServerEnv() ??
  'postgres://postgres:postgres@localhost:5432/postgres';

/** Prefix every worker database name carries, so a sweep can find them regardless of worker count. */
export const STALE_DATABASE_PREFIX = 'firebird_e2e_w';

/** A worker's own connection string — same host/port/credentials as `PLAYWRIGHT_PG_ADMIN_URL`,
 * pointed at its own database instead of the `postgres` maintenance one. */
export const databaseUrlFor = (name: string): string => {
  const url = new URL(ADMIN_URL);
  url.pathname = `/${name}`;
  return url.toString();
};

const SAFE_NAME_PATTERN = /^[a-z0-9_]+$/;

/** Database names here are always our own `firebird_e2e_w<n>` construction, never user input —
 * this is a defensive check against a future edit accidentally interpolating something else. */
const assertSafeName = (name: string): void => {
  if (!SAFE_NAME_PATTERN.test(name)) {
    throw new Error(
      `refusing to operate on suspicious database name: "${name}"`,
    );
  }
};

const withAdminClient = async <T>(
  fn: (client: Client) => Promise<T>,
): Promise<T> => {
  const client = new Client({ connectionString: ADMIN_URL });
  await client.connect();
  try {
    return await fn(client);
  } finally {
    await client.end();
  }
};

export const createWorkerDatabase = async (name: string): Promise<void> => {
  assertSafeName(name);
  await withAdminClient((client) => client.query(`CREATE DATABASE "${name}"`));
};

/**
 * `WITH (FORCE)` (Postgres 13+) also disconnects any lingering backend connections instead of
 * failing — the safety net for dropping a database left over from a crashed prior run.
 */
export const dropWorkerDatabase = async (name: string): Promise<void> => {
  assertSafeName(name);
  await withAdminClient((client) =>
    client.query(`DROP DATABASE IF EXISTS "${name}" WITH (FORCE)`),
  );
};

/**
 * Finds and drops every `firebird_e2e_w*` database, regardless of how many workers created them —
 * cleans up orphans from a crashed run even if this run uses a different `--workers` count.
 */
export const sweepStaleDatabases = async (): Promise<void> => {
  await withAdminClient(async (client) => {
    const { rows } = await client.query<{ datname: string }>(
      'SELECT datname FROM pg_database WHERE datname LIKE $1',
      [`${STALE_DATABASE_PREFIX}%`],
    );
    for (const { datname } of rows) {
      assertSafeName(datname);
      await client.query(`DROP DATABASE IF EXISTS "${datname}" WITH (FORCE)`);
    }
  });
};

/**
 * One pooled connection per worker database, reused across every test in that worker (tests in a
 * single worker run sequentially, so a single connection is enough) rather than reconnecting for
 * every `resetTransactions()` call.
 */
const transactionPools = new Map<string, Pool>();

const poolFor = (databaseUrl: string): Pool => {
  let pool = transactionPools.get(databaseUrl);
  if (!pool) {
    pool = new Pool({ connectionString: databaseUrl, max: 1 });
    transactionPools.set(databaseUrl, pool);
  }
  return pool;
};

/**
 * `transactions` has no `household_id`/`user_id` yet (that's landing in a later migration), so
 * every test in a worker shares the same rows regardless of which fresh user/household it made —
 * called before each test to keep list/sort/search assertions deterministic.
 */
export const truncateTransactions = async (
  databaseUrl: string,
): Promise<void> => {
  await poolFor(databaseUrl).query(
    'TRUNCATE TABLE transactions RESTART IDENTITY',
  );
};

export const closeTransactionPool = async (
  databaseUrl: string,
): Promise<void> => {
  const pool = transactionPools.get(databaseUrl);
  if (!pool) return;
  transactionPools.delete(databaseUrl);
  await pool.end();
};
