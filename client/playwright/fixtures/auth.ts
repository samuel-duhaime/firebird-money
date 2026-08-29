import { randomUUID } from 'node:crypto';
import type { Page } from '@playwright/test';
import { test as workerInfraTest } from './worker-infra';

type AuthFixtures = {
  authedPage: Page;
};

/**
 * A signed-in `page`, ready to `.goto('/transactions')` (or any other authed route) — built on
 * the dev/test shortcut `SKIP_EMAIL_VERIFICATION=true` gives every worker's server
 * (`POST /auth/request-login` signs in immediately and sets the session cookie on that same
 * response, no magic-link email needed).
 *
 * Uses `context.request` rather than a bare `request` fixture specifically because it shares the
 * `BrowserContext`'s cookie jar: the `Set-Cookie: session=...` response is stored automatically,
 * so the subsequent `page.goto()` a spec does carries it with no manual cookie handling.
 *
 * Deliberately does not navigate anywhere itself, so it stays reusable for other authed pages
 * later — each spec file does its own `page.goto(...)`.
 */
export const test = workerInfraTest.extend<AuthFixtures>({
  authedPage: async ({ page, context, workerInfra }, use, testInfo) => {
    await workerInfra.resetTransactions();

    const email = `e2e-${testInfo.workerIndex}-${randomUUID()}@example.test`;
    await context.request.post(`${workerInfra.apiOrigin}/auth/request-login`, {
      data: { email },
    });
    // A fresh user has no household, which routes real sign-ins to /onboarding — creating one
    // here matches what every real user journey does before reaching /transactions.
    await context.request.post(`${workerInfra.apiOrigin}/auth/onboarding`, {
      data: { join_code: null },
    });

    await use(page);
  },
});

export { expect } from './worker-infra';
