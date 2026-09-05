import { test as base } from '@playwright/test';
import type { ChildProcess } from 'node:child_process';
import {
  closeTransactionPool,
  createWorkerDatabase,
  databaseUrlFor,
  dropWorkerDatabase,
  truncateTransactions,
} from '../lib/db';
import { serverPortFor, vitePortFor } from '../lib/ports';
import {
  killTree,
  spawnServer,
  spawnVite,
  waitForPort,
} from '../lib/processes';

export type WorkerInfra = {
  databaseName: string;
  databaseUrl: string;
  apiOrigin: string;
  clientOrigin: string;
  /** Truncates `transactions` — needed before each test since the table has no per-user scoping
   * yet, so every test in this worker otherwise shares whatever earlier tests inserted. */
  resetTransactions: () => Promise<void>;
};

type WorkerFixtures = {
  workerInfra: WorkerInfra;
};

/**
 * One throwaway Postgres database + one Actix server instance + one Vite dev server, all scoped
 * to a single Playwright worker process and reused by every test that lands on it. This is what
 * makes cross-worker parallelism safe without any changes to the (currently unscoped)
 * `transactions` table: two workers never touch the same database.
 */
export const test = base.extend<{}, WorkerFixtures>({
  workerInfra: [
    // Playwright statically parses this signature and requires an object-destructuring pattern
    // for the fixtures parameter, even with none declared.
    async ({}, use, workerInfo) => {
      const idx = workerInfo.parallelIndex;
      const databaseName = `firebird_e2e_w${idx}`;
      // Same host/port/credentials as the admin connection, just pointed at this worker's own
      // database — not a hardcoded "postgres:postgres", which only happens to match CI's service.
      const databaseUrl = databaseUrlFor(databaseName);
      const serverPort = serverPortFor(idx);
      const vitePort = vitePortFor(idx);
      const apiOrigin = `http://127.0.0.1:${serverPort}`;
      const clientOrigin = `http://127.0.0.1:${vitePort}`;

      // Drop-before-create clears any leftover from a crashed prior run using this same slot.
      await dropWorkerDatabase(databaseName);
      await createWorkerDatabase(databaseName);

      let server: ChildProcess | undefined;
      let vite: ChildProcess | undefined;

      try {
        server = spawnServer({
          port: serverPort,
          databaseUrl,
          clientOrigin,
          label: `server:w${idx}`,
        });
        await waitForPort('127.0.0.1', serverPort);

        vite = spawnVite({
          port: vitePort,
          apiOrigin,
          label: `vite:w${idx}`,
        });
        await waitForPort('127.0.0.1', vitePort);
      } catch (error) {
        // A timed-out waitForPort otherwise skips `use(...)` and the teardown below entirely,
        // leaking whatever of the server/Vite/database this worker already stood up.
        if (vite) await killTree(vite);
        if (server) await killTree(server);
        await dropWorkerDatabase(databaseName);
        throw error;
      }

      await use({
        databaseName,
        databaseUrl,
        apiOrigin,
        clientOrigin,
        resetTransactions: () => truncateTransactions(databaseUrl),
      });

      await killTree(vite);
      // The server must fully exit (releasing its Postgres connections) before the DB is dropped.
      await killTree(server);
      await closeTransactionPool(databaseUrl);
      await dropWorkerDatabase(databaseName);
    },
    { scope: 'worker', timeout: 60_000 },
  ],

  // Overrides Playwright's built-in `baseURL` option so `page.goto('/relative')` and
  // `context.request` calls in spec files resolve against this worker's own Vite origin, with no
  // per-spec setup. Deliberately not using the `webServer` config option — it starts exactly one
  // shared server for the whole run, incompatible with per-worker database isolation.
  baseURL: async ({ workerInfra }, use) => {
    await use(workerInfra.clientOrigin);
  },
});

export { expect } from '@playwright/test';
