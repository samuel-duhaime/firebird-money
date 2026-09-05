import { spawn } from 'node:child_process';
import type { ChildProcess } from 'node:child_process';
import { createConnection } from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '../../..');
const serverDir = path.join(repoRoot, 'server');
const clientDir = path.resolve(here, '../..');

const SERVER_BINARY = path.join(serverDir, 'target/debug/server');
// Invoking the local binary directly (not `npx vite`) skips npx's resolution overhead and avoids
// spawning an extra wrapper process, so `killTree` only ever has one real process to stop.
const VITE_BINARY = path.join(clientDir, 'node_modules/.bin/vite');

/** Forwards a child's stdout/stderr to this process's, prefixed, so a startup failure is visible
 * in the Playwright run's own output instead of silently swallowed. */
const forwardOutput = (child: ChildProcess, label: string): void => {
  child.stdout?.on('data', (chunk: Buffer) =>
    process.stdout.write(`[${label}] ${chunk}`),
  );
  child.stderr?.on('data', (chunk: Buffer) =>
    process.stderr.write(`[${label}] ${chunk}`),
  );
};

export type SpawnServerOptions = {
  port: number;
  databaseUrl: string;
  clientOrigin: string;
  label: string;
};

export const spawnServer = ({
  port,
  databaseUrl,
  clientOrigin,
  label,
}: SpawnServerOptions): ChildProcess => {
  const child = spawn(SERVER_BINARY, [], {
    cwd: serverDir,
    env: {
      ...process.env,
      PORT: String(port),
      DATABASE_URL: databaseUrl,
      APP_ENV: 'development',
      SKIP_EMAIL_VERIFICATION: 'true',
      // Load-bearing: must be the literal "127.0.0.1", never "localhost". The session cookie is
      // host-matched, and "localhost" can resolve to ::1 on some systems — a different host that
      // would silently break cookie sharing with the Vite origin below.
      CLIENT_BASE_URL: clientOrigin,
      DEFAULT_LANGUAGE: 'en',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  forwardOutput(child, label);
  return child;
};

export type SpawnViteOptions = {
  port: number;
  apiOrigin: string;
  label: string;
};

export const spawnVite = ({
  port,
  apiOrigin,
  label,
}: SpawnViteOptions): ChildProcess => {
  const child = spawn(
    VITE_BINARY,
    // Same "always 127.0.0.1, never localhost" reasoning as spawnServer's CLIENT_BASE_URL above.
    ['--port', String(port), '--strictPort', '--host', '127.0.0.1'],
    {
      cwd: clientDir,
      env: { ...process.env, VITE_API_BASE_URL: apiOrigin },
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  forwardOutput(child, label);
  return child;
};

export type WaitForPortOptions = {
  timeoutMs?: number;
  intervalMs?: number;
};

/** Raw TCP-connect retry loop — sufficient readiness signal since the server doesn't bind until
 * its startup migrations finish, and Vite doesn't listen until it's ready to serve. No dedicated
 * `/health` endpoint needed. */
export const waitForPort = (
  host: string,
  port: number,
  { timeoutMs = 30_000, intervalMs = 200 }: WaitForPortOptions = {},
): Promise<void> =>
  new Promise((resolve, reject) => {
    const deadline = Date.now() + timeoutMs;

    const attempt = () => {
      const socket = createConnection({ host, port });

      const retryOrFail = () => {
        socket.destroy();
        if (Date.now() >= deadline) {
          reject(
            new Error(
              `nothing listening on ${host}:${port} after ${timeoutMs}ms — check DATABASE_URL/migrations or the Vite dev server's own startup log above`,
            ),
          );
          return;
        }
        setTimeout(attempt, intervalMs);
      };

      socket.once('connect', () => {
        socket.end();
        resolve();
      });
      socket.once('error', retryOrFail);
      socket.setTimeout(intervalMs, retryOrFail);
    };

    attempt();
  });

/** SIGTERM, escalating to SIGKILL if the process hasn't exited within `graceMs`. Resolves only
 * once the process has actually exited (not just once the signal was sent), so a caller can rely
 * on its resources — e.g. Postgres connections — being released afterward. */
export const killTree = (child: ChildProcess, graceMs = 5000): Promise<void> =>
  new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode !== null) {
      resolve();
      return;
    }

    child.once('exit', () => {
      clearTimeout(killTimer);
      resolve();
    });

    child.kill('SIGTERM');
    const killTimer = setTimeout(() => child.kill('SIGKILL'), graceMs);
  });
