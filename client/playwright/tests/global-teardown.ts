import { sweepStaleDatabases } from '../lib/db';

/** Final safety net: each worker drops its own database on teardown, but a worker that crashed
 * mid-run wouldn't have — clean up anything left over so a future run starts from a clean slate. */
export default async function globalTeardown(): Promise<void> {
  await sweepStaleDatabases();
}
