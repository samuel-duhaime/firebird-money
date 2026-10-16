import os from 'node:os';
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './playwright',
  globalSetup: './playwright/tests/global-setup.ts',
  globalTeardown: './playwright/tests/global-teardown.ts',
  outputDir: './playwright/test-results',
  // Parallelism happens across worker processes, each with its own fully independent
  // DB/server/Vite instance (see fixtures/worker-infra.ts). Every test signs in as its own fresh
  // user and `authedPage` truncates `transactions` first, so no test depends on another — even
  // within one file — and a big spec like transactions-page.spec.ts can spread over every worker.
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  // The README screenshot test rewrites docs/images/ when it runs, so a plain `playwright test`
  // leaves it out; `npm run screenshot:readme` opts back in.
  grepInvert: process.env.README_SCREENSHOT ? undefined : /@screenshot/,
  // Unlike a typical Playwright suite, a "worker" here owns a whole Postgres database plus an
  // Actix process plus a Vite process — a standard 2-vCPU GitHub Actions runner can't sanely
  // scale past its core count. Locally, half the cores leaves room for each worker's server and
  // Vite alongside its browser. Override either with `--workers`.
  workers: process.env.CI
    ? 2
    : Math.max(1, Math.floor(os.availableParallelism() / 2)),
  reporter: [
    ['list'],
    [
      'html',
      { open: 'always', outputFolder: './playwright/playwright-report' },
    ],
  ],
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  // Chromium only for now; widen to firefox/webkit later if the suite proves stable.
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
