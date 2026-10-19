import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const clientDir = path.resolve(here, '../..');
const VITE_BINARY = path.join(clientDir, 'node_modules/.bin/vite');

/** Under `playwright/`, next to test-results, and gitignored along with them. */
const BUILD_ROOT = path.join(clientDir, 'playwright/.app-build');
const SHARED_BUILD_DIR = path.join(BUILD_ROOT, 'shared');

/**
 * `VITE_API_BASE_URL` is baked into the bundle at build time, but every worker talks to its own
 * server port — so the suite builds once with this placeholder and each worker swaps in its own
 * origin (`prepareWorkerApp`). Distinctive enough never to occur in the bundle by accident.
 */
const API_ORIGIN_PLACEHOLDER = 'http://e2e-api-origin.invalid';

/**
 * A production build of the app, made once per run in global setup. Workers then serve it with
 * `vite preview` instead of each running a dev server that transforms every module on first load.
 * Skips `tsc -b` (type checking isn't what this suite tests) to keep it fast.
 */
export const buildApp = (): Promise<void> =>
  new Promise((resolve, reject) => {
    fs.rmSync(BUILD_ROOT, { recursive: true, force: true });
    const child = spawn(
      VITE_BINARY,
      [
        'build',
        '--outDir',
        SHARED_BUILD_DIR,
        '--emptyOutDir',
        '--logLevel',
        'warn',
      ],
      {
        cwd: clientDir,
        // A variable already set in the environment wins over `client/.env`.
        env: { ...process.env, VITE_API_BASE_URL: API_ORIGIN_PLACEHOLDER },
        stdio: 'inherit',
      },
    );
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`vite build exited with code ${code}`));
    });
  });

/** Where one worker's copy of the build lives — known before `prepareWorkerApp` runs, so a
 * failed preparation can still clean up a partial copy. */
export const workerAppDir = (label: string): string =>
  path.join(BUILD_ROOT, label);

/** Copies the shared build into `outDir` with this worker's API origin in place of the
 * placeholder. */
export const prepareWorkerApp = (outDir: string, apiOrigin: string): void => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.cpSync(SHARED_BUILD_DIR, outDir, { recursive: true });

  let replaced = false;
  for (const entry of fs.readdirSync(outDir, {
    recursive: true,
    withFileTypes: true,
  })) {
    if (!entry.isFile() || !entry.name.endsWith('.js')) continue;
    const file = path.join(entry.parentPath, entry.name);
    const source = fs.readFileSync(file, 'utf8');
    if (source.includes(API_ORIGIN_PLACEHOLDER)) {
      fs.writeFileSync(
        file,
        source.replaceAll(API_ORIGIN_PLACEHOLDER, apiOrigin),
      );
      replaced = true;
    }
  }
  // Without this, a build that somehow lost the placeholder would send every API call to the
  // wrong origin and fail in far more confusing ways.
  if (!replaced) {
    throw new Error(
      `no "${API_ORIGIN_PLACEHOLDER}" found in the app build — did VITE_API_BASE_URL change?`,
    );
  }
};

export const removeWorkerApp = (outDir: string): void => {
  fs.rmSync(outDir, { recursive: true, force: true });
};
