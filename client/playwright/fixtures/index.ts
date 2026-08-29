/** Every spec file imports `test`/`expect` from here — it pulls in full per-worker isolation
 * (`worker-infra.ts`) and a signed-in `authedPage` (`auth.ts`) with a single import. */
export { test, expect } from './auth';
export type { WorkerInfra } from './worker-infra';
