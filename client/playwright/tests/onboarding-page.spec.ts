import { randomUUID } from 'node:crypto';
import type { APIRequestContext, BrowserContext, Page } from '@playwright/test';
import { test, expect, type WorkerInfra } from '../fixtures';

/** Signs `context` in as a brand-new user who hasn't started onboarding, and opens the app. */
const signInFresh = async (
  page: Page,
  context: BrowserContext,
  workerInfra: WorkerInfra,
  workerIndex: number,
) => {
  const email = `e2e-${workerIndex}-${randomUUID()}@example.test`;
  await context.request.post(`${workerInfra.apiOrigin}/auth/request-login`, {
    data: { email },
  });
  await page.goto('/dashboard');
  return email;
};

/** Onboards someone else (own cookie jar, via `request`) into a new household, returning its
 * join code. */
const createOtherHousehold = async (
  request: APIRequestContext,
  workerInfra: WorkerInfra,
  workerIndex: number,
) => {
  await request.post(`${workerInfra.apiOrigin}/auth/request-login`, {
    data: { email: `e2e-${workerIndex}-${randomUUID()}@example.test` },
  });
  const response = await request.post(
    `${workerInfra.apiOrigin}/auth/onboarding`,
    { data: { first_name: 'Marie', last_name: 'Manager', join_code: null } },
  );
  const session = await response.json();
  return session.household.join_code as string;
};

test.describe('Onboarding', () => {
  test.beforeEach(async ({ workerInfra }) => {
    await workerInfra.resetTransactions();
  });

  test('sends a user who has not finished onboarding back to it, from any page', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await expect(page).toHaveURL('/onboarding');

    for (const path of ['/transactions', '/settings/members']) {
      await page.goto(path);
      await expect(page).toHaveURL('/onboarding');
    }
  });

  test('asks for a name, then creates a household', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);

    await page.getByLabel('First name').fill('Jane');
    await page.getByLabel('Last name (optional)').fill('Tremblay');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page
      .getByRole('button', { name: 'Create a household', exact: true })
      .click();

    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('.left-menu-profile-name')).toHaveText('Jane');

    await page.goto('/settings/members');
    const row = page.locator('li.members-list-row');
    await expect(row).toContainText('Jane Tremblay');
    await expect(row).toContainText('Manager');
  });

  test('the last name is optional', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);

    await page.getByLabel('First name').fill('Jane');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page
      .getByRole('button', { name: 'Create a household', exact: true })
      .click();

    await expect(page).toHaveURL('/dashboard');
    await page.goto('/settings/members');
    await expect(page.locator('.members-list-name')).toHaveText(/^Jane/);
  });

  test('requires a first name that is not just spaces', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);

    await page.getByLabel('First name').fill('   ');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();

    await expect(page.getByText('A first name is required.')).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Create a household', exact: true }),
    ).not.toBeVisible();
  });

  test('Back returns to the name step with the name kept', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);

    await page.getByLabel('First name').fill('Jane');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page.getByRole('button', { name: 'Back', exact: true }).click();

    await expect(page.getByLabel('First name')).toHaveValue('Jane');
  });

  test('joins an existing household with its join code', async ({
    page,
    context,
    request,
    workerInfra,
  }, testInfo) => {
    const joinCode = await createOtherHousehold(
      request,
      workerInfra,
      testInfo.workerIndex,
    );
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);

    await page.getByLabel('First name').fill('Jane');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page
      .getByRole('button', { name: 'I have a join code', exact: true })
      .click();
    await page.getByPlaceholder('Join code').fill(joinCode.toLowerCase());
    await page
      .getByRole('button', { name: 'Join household', exact: true })
      .click();

    await expect(page).toHaveURL('/dashboard');
    await page.goto('/settings/members');
    await expect(page.locator('li.members-list-row')).toHaveCount(2);
    await expect(
      page.locator('li.members-list-row', { hasText: 'Marie Manager' }),
    ).toContainText('Manager');
    await expect(page.locator('.invite-code-value')).toHaveText(joinCode);
  });

  test('says so when no household matches the join code', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);

    await page.getByLabel('First name').fill('Jane');
    await page.getByRole('button', { name: 'Continue', exact: true }).click();
    await page
      .getByRole('button', { name: 'I have a join code', exact: true })
      .click();
    await page.getByPlaceholder('Join code').fill('ZZZZZZZZ');
    await page
      .getByRole('button', { name: 'Join household', exact: true })
      .click();

    await expect(
      page.getByText('No household matches this join code.'),
    ).toBeVisible();
    await expect(page).toHaveURL('/onboarding');
  });

  test('a finished user is sent from onboarding to the dashboard', async ({
    authedPage,
  }) => {
    await authedPage.goto('/onboarding');
    await expect(authedPage).toHaveURL('/dashboard');
  });
});
