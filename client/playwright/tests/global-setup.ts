import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { sweepStaleDatabases } from '../lib/db';

const here = path.dirname(fileURLToPath(import.meta.url));
const serverDir = path.resolve(here, '../../../server');

/** Runs once for the whole suite, before any worker starts (Playwright guarantees this) — so N
 * workers never independently invoke `cargo build` and contend on `target/`. Cargo's own
 * incremental build makes this a no-op if nothing changed, so it stays cheap on repeat local runs. */
const buildServerBinary = (): Promise<void> =>
  new Promise((resolve, reject) => {
    const child = spawn('cargo', ['build', '--bin', 'server'], {
      cwd: serverDir,
      stdio: 'inherit',
    });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) resolve();
      else
        reject(new Error(`cargo build --bin server exited with code ${code}`));
    });
  });

export default async function globalSetup(): Promise<void> {
  await buildServerBinary();
  // Defensive pre-clean: a previous run that crashed (Ctrl+C, OOM kill, ...) may have left worker
  // databases behind. Sweeping by prefix rather than a fixed worker count handles a prior run
  // that used a different --workers value than this one.
  await sweepStaleDatabases();
}
