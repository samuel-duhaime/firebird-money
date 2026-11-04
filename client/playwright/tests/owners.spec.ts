import type { APIRequestContext, Locator, Page } from '@playwright/test';
import { test, expect } from '../fixtures';
import { createMerchant, ensureAccount, getCategoryId } from '../lib/seed';

/** A household of two: the signed-in Test User and Alice Gaudreau, who joins with the household's
 * code from her own session. Returns Alice's user id. */
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

/** An account named `name`, owned by `owner` (a member id or `'shared'`). */
const ownedAccount = async (
  request: APIRequestContext,
  apiOrigin: string,
  name: string,
  owner: number | 'shared',
): Promise<number> => {
  const id = await ensureAccount(request, apiOrigin, name);
  const response = await request.patch(`${apiOrigin}/accounts/${id}`, {
    data: { owner },
  });
  expect(response.ok()).toBe(true);
  return id;
};

/** Adds a transaction at `merchant` in `accountId`, returning its id. */
const addTransaction = async (
  request: APIRequestContext,
  apiOrigin: string,
  accountId: number,
  merchant: string,
): Promise<number> => {
  const response = await request.post(`${apiOrigin}/transactions`, {
    data: {
      date: '2026-10-26',
      original_statement: merchant,
      amount: '233.62',
      category_id: await getCategoryId(request, apiOrigin, 'Groceries'),
      account_id: accountId,
    },
  });
  expect(response.ok()).toBe(true);
  return ((await response.json()) as { id: number }).id;
};

const ownerOf = async (
  request: APIRequestContext,
  apiOrigin: string,
  transactionId: number,
): Promise<number | null> =>
  (
    await (
      await request.get(`${apiOrigin}/transactions/${transactionId}`)
    ).json()
  ).owner_user_id;

/** The name an Owner field shows, without its avatar's letter. */
const ownerName = (owner: Locator) => owner.locator('.owner-picker-name');

/** Picks `owner` ("Shared" or a member's name, with any "(default)") from an open Owner list. */
const pickOwner = (page: Page, owner: string) =>
  page.getByRole('option', { name: owner, exact: true }).click();

test.describe('Transaction owners', () => {
  test('adding a transaction: Owner waits for an account, then takes its owner', async ({
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
    await ownedAccount(context.request, apiOrigin, 'Visa Alice', aliceId);
    await ownedAccount(context.request, apiOrigin, 'Joint', 'shared');
    await createMerchant(context.request, apiOrigin, 'Corner Store');

    await authedPage.goto('/transactions/add-transaction');
    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    const owner = dialog.getByLabel('Owner');
    await expect(owner).toBeDisabled();
    await expect(owner).toHaveText('Select an account first');

    await dialog
      .getByLabel('Account')
      .selectOption({ label: 'Seed Bank — Visa Alice' });
    await expect(owner).toBeEnabled();
    await expect(ownerName(owner)).toHaveText('Alice Gaudreau (default)');

    // Another account brings its own owner.
    await dialog
      .getByLabel('Account')
      .selectOption({ label: 'Seed Bank — Joint' });
    await expect(ownerName(owner)).toHaveText('Shared (default)');

    await owner.click();
    await pickOwner(authedPage, 'Alice Gaudreau');
    await expect(ownerName(owner)).toHaveText('Alice Gaudreau');

    // Saved with the owner picked, not the Joint account's.
    await dialog.getByLabel('Amount').fill('12.50');
    await dialog.getByRole('button', { name: 'Merchant', exact: true }).click();
    await authedPage
      .locator('.merchant-picker-popover')
      .getByRole('button', { name: 'Corner Store', exact: true })
      .click();
    await dialog.getByLabel('Date').fill('2026-10-26');
    await authedPage
      .getByRole('button', { name: 'Select category', exact: true })
      .click();
    await authedPage
      .locator('.category-picker-popover')
      .getByRole('button', { name: 'Groceries', exact: true })
      .click();
    await dialog
      .getByRole('button', { name: 'Add transaction', exact: true })
      .click();
    await expect(dialog).not.toBeVisible();

    const transactions: { merchant_name: string; owner_user_id: number }[] =
      await (await context.request.get(`${apiOrigin}/transactions`)).json();
    expect(
      transactions.find((t) => t.merchant_name === 'Corner Store')
        ?.owner_user_id,
    ).toBe(aliceId);
  });

  test('the edit panel marks the default owner and saves another', async ({
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
    const visa = await ownedAccount(
      context.request,
      apiOrigin,
      'Visa Alice',
      aliceId,
    );
    const id = await addTransaction(
      context.request,
      apiOrigin,
      visa,
      'Tangerine',
    );

    await authedPage.goto(`/transactions/${id}`);
    const owner = authedPage
      .getByRole('dialog', { name: 'Edit transaction' })
      .getByLabel('Owner');
    await expect(ownerName(owner)).toHaveText('Alice Gaudreau (default)');

    await owner.click();
    await pickOwner(authedPage, 'Shared');
    await expect(ownerName(owner)).toHaveText('Shared');
    await expect.poll(() => ownerOf(context.request, apiOrigin, id)).toBeNull();
  });

  test('the Owner column shows each owner, with name and email on hover', async ({
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
    const visa = await ownedAccount(
      context.request,
      apiOrigin,
      'Visa Alice',
      aliceId,
    );
    const id = await addTransaction(
      context.request,
      apiOrigin,
      visa,
      'Tangerine',
    );

    await authedPage.goto('/transactions');
    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Tangerine',
    });
    const cell = row.locator('.transactions-row-owner');
    // Shown by default, and hidden like the other columns.
    await expect(cell).toBeVisible();
    await authedPage
      .getByRole('button', { name: 'Columns', exact: true })
      .click();
    const ownerSwitch = authedPage.getByRole('switch', { name: 'Owner' });
    await expect(ownerSwitch).toBeChecked();
    // Flips once the setting is saved.
    await ownerSwitch.click();
    await expect(ownerSwitch).not.toBeChecked();
    await expect(cell).toHaveCount(0);
    await ownerSwitch.click();
    await expect(ownerSwitch).toBeChecked();
    await authedPage.keyboard.press('Escape');
    await expect(cell).toBeVisible();

    await cell.hover();
    await expect(authedPage.locator('.tooltip')).toHaveText(
      /Alice Gaudreau \(default\)\s*alice-\d+@example\.test/,
    );

    await cell.getByRole('combobox').click();
    await pickOwner(authedPage, 'Shared');
    await expect.poll(() => ownerOf(context.request, apiOrigin, id)).toBeNull();
  });

  test('Edit multiple sets the owner of every selected transaction', async ({
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
    const joint = await ownedAccount(
      context.request,
      apiOrigin,
      'Joint',
      'shared',
    );
    const ids = [
      await addTransaction(context.request, apiOrigin, joint, 'Tangerine'),
      await addTransaction(context.request, apiOrigin, joint, 'Desjardins'),
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
    await panel.getByLabel('Owner').click();
    await pickOwner(authedPage, 'Alice Gaudreau');
    await panel.getByRole('button', { name: 'Save', exact: true }).click();

    for (const id of ids) {
      await expect
        .poll(() => ownerOf(context.request, apiOrigin, id))
        .toBe(aliceId);
    }
  });
});

test.describe('Edit account owners', () => {
  test('sets many accounts at once, and their transactions follow', async ({
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
    const visa = await ownedAccount(
      context.request,
      apiOrigin,
      'Visa',
      'shared',
    );
    const joint = await ownedAccount(
      context.request,
      apiOrigin,
      'Joint',
      'shared',
    );
    const id = await addTransaction(
      context.request,
      apiOrigin,
      visa,
      'Tangerine',
    );

    await authedPage.goto('/settings/accounts');
    await authedPage
      .getByRole('button', { name: 'Edit owners', exact: true })
      .click();
    const panel = authedPage.getByRole('dialog', {
      name: 'Edit account owners',
    });
    await expect(
      panel.getByText('Its transactions take the same owner', { exact: false }),
    ).toBeVisible();

    const visaOwner = panel.getByRole('radiogroup', { name: 'Owner: Visa' });
    await expect(
      visaOwner.getByRole('radio', { name: 'Shared' }),
    ).toBeChecked();
    await visaOwner.getByRole('radio', { name: 'Alice Gaudreau' }).click();
    await expect(
      visaOwner.getByRole('radio', { name: 'Alice Gaudreau' }),
    ).toBeChecked();

    // Cancel keeps everything as it was.
    await panel.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(panel).not.toBeVisible();
    expect(await ownerOf(context.request, apiOrigin, id)).toBeNull();

    await authedPage
      .getByRole('button', { name: 'Edit owners', exact: true })
      .click();
    await visaOwner.getByRole('radio', { name: 'Alice Gaudreau' }).click();
    await panel
      .getByRole('radiogroup', { name: 'Owner: Joint' })
      .getByRole('radio', { name: 'Alice Gaudreau' })
      .click();
    await panel.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(authedPage.getByText('Account owners saved.')).toBeVisible();
    await expect(panel).not.toBeVisible();
    await expect(
      authedPage.locator('li.institutions-list-row', { hasText: 'Visa' }),
    ).toContainText('Alice Gaudreau');
    expect(await ownerOf(context.request, apiOrigin, id)).toBe(aliceId);
    const joined: { owner_user_id: number | null } = await (
      await context.request.get(`${apiOrigin}/accounts/${joint}`)
    ).json();
    expect(joined.owner_user_id).toBe(aliceId);
  });
});
