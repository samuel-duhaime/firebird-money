/**
 * Deterministic port allocation, keyed on Playwright's `parallelIndex` (a stable 0..workers-1
 * slot id — unlike `workerIndex`, it doesn't increment when a crashed worker is replaced). Kept
 * far from the real dev ports (3055/5173) so this never collides with a developer's own running
 * app.
 */
const SERVER_PORT_BASE = 4100;
const VITE_PORT_BASE = 4300;

export const serverPortFor = (parallelIndex: number): number =>
  SERVER_PORT_BASE + parallelIndex;

export const vitePortFor = (parallelIndex: number): number =>
  VITE_PORT_BASE + parallelIndex;
