import type { APIRequestContext, Page } from '@playwright/test';
import { test, expect } from '../fixtures';
import { ensureAccount, getCategoryId } from '../lib/seed';

/** Alice Gaudreau joins the signed-in Test User's household. Returns her user id. */
const addAlice = async (
  request: APIRequestContext,
  newContext: () => Promise<APIRequestContext>,
  apiOrigin: string,
): Promise<number> => {
  const me = await (await request.get(`${apiOrigin}/auth/me`)).json();
  const alice = await newContext();
  await alice.post(`${apiOrigin}/auth/request-login`, {
    data: { email: `alice-${me.user.id}@example.test` },
  });
  const onboarding = await alice.post(`${apiOrigin}/auth/onboarding`, {
    data: {
      first_name: 'Alice',
      last_name: 'Gaudreau',
      join_code: me.household.join_code,
    },
  });
  expect(onboarding.ok()).toBe(true);
  const id: number = (await (await alice.get(`${apiOrigin}/auth/me`)).json())
    .user.id;
  await alice.dispose();
  return id;
};

/** Adds a transaction at `merchant`, waiting on `assignee` for a review when given (else reviewed,
 * as every new transaction starts). Returns its id. */
const addTransaction = async (
  request: APIRequestContext,
  apiOrigin: string,
  merchant: string,
  assignee?: number | 'anyone',
): Promise<number> => {
  const response = await request.post(`${apiOrigin}/transactions`, {
    data: {
      date: '2026-10-26',
      original_statement: merchant,
      amount: '233.62',
      category_id: await getCategoryId(request, apiOrigin, 'Groceries'),
      account_id: await ensureAccount(request, apiOrigin, 'Visa'),
    },
  });
  expect(response.ok()).toBe(true);
  const id = ((await response.json()) as { id: number }).id;
  if (assignee !== undefined) {
    const patched = await request.patch(`${apiOrigin}/transactions/${id}`, {
      data: { review_assignee: assignee },
    });
    expect(patched.ok()).toBe(true);
  }
  return id;
};

/** Picks `name` from an open "Needs review by" list. */
const pick = (page: Page, name: string) =>
  page.getByRole('listbox').getByRole('option', { name, exact: true }).click();

const isReviewed = async (
  request: APIRequestContext,
  apiOrigin: string,
  id: number,
): Promise<boolean> =>
  (await (await request.get(`${apiOrigin}/transactions/${id}`)).json())
    .reviewed;

test.describe('Transaction reviews', () => {
  test('the review views list what waits, and Mark all reviewed empties them', async ({
    authedPage,
    context,
    playwright,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    const aliceId = await addAlice(
      context.request,
      () => playwright.request.newContext(),
      apiOrigin,
    );
    await addTransaction(context.request, apiOrigin, 'Tangerine');
    await addTransaction(context.request, apiOrigin, 'Desjardins', 'anyone');
    await addTransaction(context.request, apiOrigin, 'Koodo', aliceId);

    const row = (merchant: string) =>
      authedPage.locator('li.transactions-row', { hasText: merchant });
    const trigger = authedPage
      .locator('.transactions-toolbar > button')
      .first();
    const pickView = async (name: RegExp) => {
      await trigger.click();
      await authedPage
        .locator('.review-view-popover')
        .getByRole('button', { name })
        .click();
    };

    await authedPage.goto('/transactions');
    await expect(row('Tangerine')).toBeVisible();
    await trigger.click();
    const popover = authedPage.locator('.review-view-popover');
    await expect(
      popover.getByRole('button', { name: /^Needs review\s*2$/ }),
    ).toBeVisible();
    await expect(
      popover.getByRole('button', { name: /^Needs my review\s*1$/ }),
    ).toBeVisible();
    await authedPage.keyboard.press('Escape');

    await pickView(/^Needs review/);
    await expect(trigger).toContainText('Needs review');
    await expect(row('Desjardins')).toBeVisible();
    await expect(row('Koodo')).toBeVisible();
    await expect(row('Tangerine')).not.toBeVisible();
    expect(new URL(authedPage.url()).searchParams.get('needs_review')).toBe(
      'all',
    );

    // One at a time: the row's own button marks it reviewed, and it leaves the view.
    await row('Koodo')
      .getByRole('button', { name: 'Mark Koodo reviewed', exact: true })
      .click();
    await expect(row('Koodo')).not.toBeVisible();
    await expect(row('Desjardins')).toBeVisible();

    // Waiting on Anyone counts as mine.
    await pickView(/^Needs my review/);
    await expect(row('Desjardins')).toBeVisible();
    await expect(
      authedPage.getByText('1 transaction is waiting on you'),
    ).toBeVisible();

    await authedPage
      .getByRole('button', { name: 'Mark all reviewed', exact: true })
      .click();
    await expect(
      authedPage.getByText('You reviewed everything!'),
    ).toBeVisible();
    await authedPage
      .getByRole('button', { name: 'View all transactions', exact: true })
      .click();
    await expect(row('Tangerine')).toBeVisible();
  });

  test('the edit panel sends it for review and marks it reviewed', async ({
    authedPage,
    context,
    playwright,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    const aliceId = await addAlice(
      context.request,
      () => playwright.request.newContext(),
      apiOrigin,
    );
    const id = await addTransaction(context.request, apiOrigin, 'Tangerine');

    await authedPage.goto(`/transactions/${id}`);
    const dialog = authedPage.getByRole('dialog', { name: 'Edit transaction' });
    const field = dialog.getByLabel('Needs review by');
    await expect(field).toHaveText('No one');

    await field.click();
    await pick(authedPage, 'Alice Gaudreau');
    await expect
      .poll(
        async () =>
          (
            await (
              await context.request.get(`${apiOrigin}/transactions/${id}`)
            ).json()
          ).review_assignee_user_id,
      )
      .toBe(aliceId);
    expect(await isReviewed(context.request, apiOrigin, id)).toBe(false);

    // Waiting on Alice, but Test User can still review it.
    await field.click();
    await pick(authedPage, 'No one');
    await expect(dialog.getByText(/^Reviewed by Test User · /)).toBeVisible();
    expect(await isReviewed(context.request, apiOrigin, id)).toBe(true);
  });

  test('Edit multiple sends every selected transaction for review', async ({
    authedPage,
    context,
    playwright,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    await addAlice(
      context.request,
      () => playwright.request.newContext(),
      apiOrigin,
    );
    const ids = [
      await addTransaction(context.request, apiOrigin, 'Tangerine'),
      await addTransaction(context.request, apiOrigin, 'Desjardins'),
    ];

    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Edit multiple', exact: true })
      .click();
    await authedPage
      .getByRole('checkbox', { name: 'All transactions (CTRL+A)' })
      .click();
    await authedPage
      .getByRole('button', { name: 'Edit 2', exact: true })
      .click();

    const panel = authedPage.getByRole('dialog', {
      name: 'Edit 2 transactions',
    });
    await panel.getByLabel('Needs review by').click();
    await pick(authedPage, 'Alice Gaudreau');
    await panel.getByRole('button', { name: 'Save', exact: true }).click();

    for (const id of ids) {
      await expect
        .poll(() => isReviewed(context.request, apiOrigin, id))
        .toBe(false);
    }
  });

  test('the Filters popover narrows by who a review waits on', async ({
    authedPage,
    context,
    playwright,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    const aliceId = await addAlice(
      context.request,
      () => playwright.request.newContext(),
      apiOrigin,
    );
    await addTransaction(context.request, apiOrigin, 'Tangerine');
    await addTransaction(context.request, apiOrigin, 'Desjardins', 'anyone');
    await addTransaction(context.request, apiOrigin, 'Koodo', aliceId);

    const row = (merchant: string) =>
      authedPage.locator('li.transactions-row', { hasText: merchant });
    const popover = authedPage.locator('.filters-popover');
    const openOther = async () => {
      await authedPage.getByRole('button', { name: /^Filters/ }).click();
      await popover.getByRole('button', { name: 'Other', exact: true }).click();
    };
    const apply = () =>
      popover.getByRole('button', { name: 'Apply', exact: true }).click();

    await authedPage.goto('/transactions');
    await expect(row('Tangerine')).toBeVisible();

    await openOther();
    await expect(popover.getByText('Needs review by')).toBeVisible();
    await popover
      .getByRole('checkbox', { name: 'Alice Gaudreau', exact: true })
      .check();
    await apply();
    await expect(row('Koodo')).toBeVisible();
    await expect(row('Desjardins')).not.toBeVisible();
    await expect(row('Tangerine')).not.toBeVisible();

    await openOther();
    await popover
      .getByRole('checkbox', { name: 'Anyone', exact: true })
      .check();
    await apply();
    await expect(row('Koodo')).toBeVisible();
    await expect(row('Desjardins')).toBeVisible();
    await expect(row('Tangerine')).not.toBeVisible();
    await expect(
      authedPage.getByRole('button', { name: 'Filters (2)' }),
    ).toBeVisible();
  });
});
