import type { Page } from '@playwright/test';
import { test, expect } from '../fixtures';
import { ensureAccount, getCategoryId, seedTransaction } from '../lib/seed';

/** The page header's "Add account" — each institution card has its own button with the same name. */
const openAddAccount = (page: Page) =>
  page
    .locator('.settings-panel-header')
    .getByRole('button', { name: 'Add account', exact: true })
    .click();

const card = (page: Page, institution: string) =>
  page.locator('section.institutions-list-card', { hasText: institution });

test.describe('Household > Institutions settings', () => {
  test('left nav lands on the Institutions page', async ({ authedPage }) => {
    await authedPage.goto('/settings/members');
    await authedPage
      .getByRole('link', { name: 'Institutions', exact: true })
      .click();

    await expect(authedPage).toHaveURL('/settings/institutions');
    await expect(
      authedPage.getByRole('heading', { name: 'Institutions', exact: true }),
    ).toBeVisible();
    await expect(
      authedPage.getByRole('link', { name: 'Institutions', exact: true }),
    ).toHaveClass(/settings-nav-link--active/);
  });

  test('shows an empty state for a household with no accounts', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/institutions');

    await expect(authedPage.getByText('No accounts yet.')).toBeVisible();
    // Common institutions only show up once the household holds an account there.
    await expect(
      authedPage.locator('section.institutions-list-card'),
    ).toHaveCount(0);
  });

  test('adds an account at a common institution, found by one of its aliases', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/institutions');
    await openAddAccount(authedPage);

    const picker = authedPage.getByRole('dialog', {
      name: 'Choose an institution',
    });
    await picker.getByLabel('Search institutions').fill('banque nat');
    await expect(
      picker.getByRole('button', { name: /Desjardins/ }),
    ).toHaveCount(0);
    await picker.getByRole('button', { name: /National Bank/ }).click();

    const form = authedPage.getByRole('dialog', { name: 'Add account' });
    await expect(form).toContainText('National Bank');
    await form.getByLabel('Account name').fill('Compte conjoint');
    await form.getByLabel('Type').selectOption({ label: 'Savings' });
    await form.getByLabel('Last 4 digits (optional)').fill('5012');
    await form
      .getByRole('button', { name: 'Add account', exact: true })
      .click();

    await expect(form).not.toBeVisible();
    const nationalBank = card(authedPage, 'National Bank');
    await expect(nationalBank).toContainText('1 account');
    await expect(nationalBank).toContainText('Compte conjoint (…5012)');
    await expect(nationalBank).toContainText('Savings · CAD · 0 transactions');
    // Common institutions can't be renamed or deleted.
    await expect(
      nationalBank
        .locator('.institutions-list-card-header')
        .getByRole('button', { name: 'Edit', exact: true }),
    ).toHaveCount(0);
  });

  test('requires an account name, and rejects a bad last four', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/institutions');
    await openAddAccount(authedPage);
    await authedPage
      .getByRole('dialog', { name: 'Choose an institution' })
      .getByRole('button', { name: /Tangerine/ })
      .click();

    const form = authedPage.getByRole('dialog', { name: 'Add account' });
    await form
      .getByRole('button', { name: 'Add account', exact: true })
      .click();
    await expect(form.getByRole('alert')).toHaveText('A name is required.');

    await form.getByLabel('Account name').fill('Épargne');
    await form.getByLabel('Last 4 digits (optional)').fill('12-4');
    await form
      .getByRole('button', { name: 'Add account', exact: true })
      .click();
    await expect(form.getByRole('alert')).toHaveText(
      'Use up to 4 letters or digits.',
    );
  });

  test('adds a custom institution, then an account in it', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/institutions');
    await openAddAccount(authedPage);

    const picker = authedPage.getByRole('dialog', {
      name: 'Choose an institution',
    });
    await picker.getByLabel('Search institutions').fill('Caisse du coin');
    await expect(picker).toContainText('No institutions found.');
    await picker
      .getByRole('button', { name: /Add your own institution/ })
      .click();

    const custom = authedPage.getByRole('dialog', {
      name: 'Add an institution',
    });
    // Carries over what was typed in the search.
    await expect(custom.getByLabel('Institution name')).toHaveValue(
      'Caisse du coin',
    );
    await custom.getByLabel('Website (optional)').fill('https://example.com');
    await custom.getByRole('button', { name: 'Continue', exact: true }).click();

    const form = authedPage.getByRole('dialog', { name: 'Add account' });
    await form.getByLabel('Account name').fill('Chèques');
    await form
      .getByRole('button', { name: 'Add account', exact: true })
      .click();

    const caisse = card(authedPage, 'Caisse du coin');
    await expect(caisse).toContainText('Chèques');
    await expect(
      caisse
        .locator('.institutions-list-card-header')
        .getByRole('button', { name: 'Edit', exact: true }),
    ).toBeVisible();
  });

  test("a card's own Add account skips the institution step", async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await ensureAccount(context.request, workerInfra.apiOrigin, 'Chequing');
    await authedPage.goto('/settings/institutions');

    await card(authedPage, 'Seed Bank')
      .getByRole('button', { name: 'Add account', exact: true })
      .click();
    const form = authedPage.getByRole('dialog', { name: 'Add account' });
    await expect(form).toContainText('Seed Bank');
    await form.getByLabel('Account name').fill('Visa');
    await form.getByLabel('Type').selectOption({ label: 'Credit card' });
    await form
      .getByRole('button', { name: 'Add account', exact: true })
      .click();

    await expect(card(authedPage, 'Seed Bank')).toContainText('2 accounts');
  });

  test('edits an account through the Edit account modal, prefilled', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await ensureAccount(context.request, workerInfra.apiOrigin, 'Chequing');
    await authedPage.goto('/settings/institutions');

    const row = authedPage.locator('li.institutions-list-row', {
      hasText: 'Chequing',
    });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();

    const form = authedPage.getByRole('dialog', { name: 'Edit account' });
    await expect(form.getByLabel('Account name')).toHaveValue('Chequing');
    await form.getByLabel('Account name').fill('Compte courant');
    await form.getByLabel('Currency').selectOption('USD');
    await form.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(form).not.toBeVisible();
    await expect(
      authedPage.locator('li.institutions-list-row', {
        hasText: 'Compte courant',
      }),
    ).toContainText('USD');
  });

  test('deleting an account asks for confirmation first', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await ensureAccount(context.request, workerInfra.apiOrigin, 'Chequing');
    await authedPage.goto('/settings/institutions');

    const row = authedPage.locator('li.institutions-list-row', {
      hasText: 'Chequing',
    });
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(row).toContainText('Delete the "Chequing" account?');
    await row.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(row).toBeVisible();

    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(row).toHaveCount(0);
  });

  test('an account still used by a transaction shows a specific error, and stays', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const categoryId = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2026-10-01',
      merchant: 'Corner Store',
      amount: '20.00',
      categoryId,
      account: 'Chequing',
    });
    await authedPage.goto('/settings/institutions');

    const row = authedPage.locator('li.institutions-list-row', {
      hasText: 'Chequing',
    });
    await expect(row).toContainText('1 transaction');
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await row.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(
      authedPage.getByText(
        "This account still has transactions, so it can't be deleted.",
      ),
    ).toBeVisible();
    await expect(row).toBeVisible();
  });

  test('a custom institution can be renamed, and deleted once it has no accounts', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await ensureAccount(context.request, workerInfra.apiOrigin, 'Chequing');
    await authedPage.goto('/settings/institutions');

    const header = card(authedPage, 'Seed Bank').locator(
      '.institutions-list-card-header',
    );
    // Still holds an account, so there's no Delete yet.
    await expect(
      header.getByRole('button', { name: 'Delete', exact: true }),
    ).toHaveCount(0);

    await header.getByRole('button', { name: 'Edit', exact: true }).click();
    const dialog = authedPage.getByRole('dialog', { name: 'Edit institution' });
    await dialog.getByLabel('Institution name').fill('Caisse renommée');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog).not.toBeVisible();

    const renamed = card(authedPage, 'Caisse renommée');
    const row = renamed.locator('li.institutions-list-row');
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(renamed).toContainText('0 accounts');

    const renamedHeader = renamed.locator('.institutions-list-card-header');
    await renamedHeader
      .getByRole('button', { name: 'Delete', exact: true })
      .click();
    await renamedHeader
      .getByRole('button', { name: 'Delete', exact: true })
      .click();
    await expect(renamed).toHaveCount(0);
  });

  test('a transaction count links to that account\'s transactions', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { request } = context;
    const api = workerInfra.apiOrigin;
    const categoryId = await getCategoryId(request, api, 'Groceries');
    await seedTransaction(request, api, {
      date: '2026-10-01',
      merchant: 'Corner Store',
      amount: '20.00',
      categoryId,
      account: 'Joint Account',
    });
    await seedTransaction(request, api, {
      date: '2026-10-02',
      merchant: 'Other Place',
      amount: '30.00',
      categoryId,
      account: 'Savings Jar',
    });
    const jointId = await ensureAccount(request, api, 'Joint Account');
    await authedPage.goto('/settings/institutions');

    await authedPage
      // Not `hasText` — every seeded account's type label reads "Chequing" too.
      .locator('li.institutions-list-row', {
        has: authedPage.getByText('Joint Account', { exact: true }),
      })
      .getByRole('link', { name: '1 transaction', exact: true })
      .click();

    await expect(authedPage).toHaveURL(`/transactions?account_ids=${jointId}`);
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Corner Store' }),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Other Place' }),
    ).not.toBeVisible();
  });
});
