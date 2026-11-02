import { randomUUID } from 'node:crypto';
import type { APIRequestContext, BrowserContext, Page } from '@playwright/test';
import { test, expect, type WorkerInfra } from '../fixtures';
import { pngFile } from '../lib/pictures';

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

/** Answers the name step, creating a household, which lands on the optional photo step. */
const reachPhotoStep = async (page: Page) => {
  await page.getByLabel('First name').fill('Jane');
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page
    .getByRole('button', { name: 'Create a household', exact: true })
    .click();
  await expect(
    page.getByRole('heading', { name: 'Add a photo' }),
  ).toBeVisible();
};

const skipPhoto = (page: Page) =>
  page.getByRole('button', { name: 'Skip for now', exact: true }).click();

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

    await skipPhoto(page);
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

    await skipPhoto(page);
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

    await skipPhoto(page);
    await expect(page).toHaveURL('/dashboard');
    await page.goto('/settings/members');
    await expect(page.locator('li.members-list-row')).toHaveCount(2);
    await expect(
      page.locator('li.members-list-row', { hasText: 'Marie Manager' }),
    ).toContainText('Manager');
    await expect(
      page
        .locator('li.members-list-row', { hasText: 'Jane' })
        .locator('.members-list-role'),
    ).toHaveText('Member');
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

  test('the photo can be skipped', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await reachPhotoStep(page);
    await expect(page.locator('.auth-card span.avatar')).toHaveText('J');

    await skipPhoto(page);

    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('.left-menu-profile span.avatar')).toHaveText(
      'J',
    );
  });

  test('a photo added during onboarding shows in the app', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await reachPhotoStep(page);

    await page.getByLabel('Choose photo').setInputFiles(await pngFile(page));

    await expect(page.locator('.auth-card img.avatar')).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Skip for now', exact: true }),
    ).toHaveCount(0);
    await page.getByRole('button', { name: 'Continue', exact: true }).click();

    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('.left-menu-profile img.avatar')).toBeVisible();
    await page.goto('/settings/members');
    await expect(page.locator('li.members-list-row img.avatar')).toBeVisible();
  });

  test('a photo can be removed again during onboarding', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await reachPhotoStep(page);
    await page.getByLabel('Choose photo').setInputFiles(await pngFile(page));
    await expect(page.locator('.auth-card img.avatar')).toBeVisible();

    await page.getByRole('button', { name: 'Remove photo' }).click();

    await expect(page.locator('.auth-card span.avatar')).toHaveText('J');
    await skipPhoto(page);
    await expect(page).toHaveURL('/dashboard');
  });

  test('someone joining a household is asked for a photo too', async ({
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
    await page.getByPlaceholder('Join code').fill(joinCode);
    await page
      .getByRole('button', { name: 'Join household', exact: true })
      .click();

    await expect(
      page.getByRole('heading', { name: 'Add a photo' }),
    ).toBeVisible();
    await page.getByLabel('Choose photo').setInputFiles(await pngFile(page));
    await expect(page.locator('.auth-card img.avatar')).toBeVisible();
  });

  test('a failed photo upload does not block finishing onboarding', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await page.route('**/members/*/picture', (route) =>
      route.fulfill({ status: 500, json: { error: 'down' } }),
    );
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await reachPhotoStep(page);

    await page.getByLabel('Choose photo').setInputFiles(await pngFile(page));

    await expect(
      page.getByText('Could not save the photo. Please try again.'),
    ).toBeVisible();
    await skipPhoto(page);
    await expect(page).toHaveURL('/dashboard');
  });

  test('leaving at the photo step does not bring anyone back to onboarding', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await reachPhotoStep(page);

    await page.reload();

    await expect(page).toHaveURL('/dashboard');
  });

  test('the photo step is translated in French', async ({
    page,
    context,
    workerInfra,
  }, testInfo) => {
    await page.addInitScript(() =>
      localStorage.setItem('firebird-language', 'fr'),
    );
    await signInFresh(page, context, workerInfra, testInfo.workerIndex);
    await page.getByLabel('Prénom').fill('Jeanne');
    await page.getByRole('button', { name: 'Continuer', exact: true }).click();
    await page
      .getByRole('button', { name: 'Créer un ménage', exact: true })
      .click();

    await expect(
      page.getByRole('heading', { name: 'Ajoute une photo' }),
    ).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Passer pour l’instant', exact: true }),
    ).toBeVisible();
  });

  test('a finished user is sent from onboarding to the dashboard', async ({
    authedPage,
  }) => {
    await authedPage.goto('/onboarding');
    await expect(authedPage).toHaveURL('/dashboard');
  });
});
