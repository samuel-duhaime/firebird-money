import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './playwright',
  globalSetup: './playwright/tests/global-setup.ts',
  globalTeardown: './playwright/tests/global-teardown.ts',
  outputDir: './playwright/test-results',
  // Parallelism happens across worker processes, each with its own fully independent
  // DB/server/Vite instance (see fixtures/worker-infra.ts). One worker per spec *file*, not per
  // test — fullyParallel would split a single file's tests across multiple workers instead.
  fullyParallel: false,
  retries: process.env.CI ? 1 : 0,
  // Unlike a typical Playwright suite, a "worker" here owns a whole Postgres database plus an
  // Actix process plus a Vite process — a standard 2-vCPU GitHub Actions runner can't sanely
  // scale past its core count under Playwright's default heuristic. Locally, there's only one
  // page's worth of specs so far, so a single worker avoids spinning up redundant DB/server/Vite
  // stacks; raise this once more page specs exist to make cross-file parallelism worthwhile.
  workers: process.env.CI ? 2 : 1,
  reporter: [
    ['list'],
    ['html', { open: 'always', outputFolder: './playwright/playwright-report' }],
  ],
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  // Chromium only for now; widen to firefox/webkit later if the suite proves stable.
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
