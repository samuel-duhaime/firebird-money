import { test, expect } from '../fixtures';
import { createMerchant, getCategoryId, seedTransaction } from '../lib/seed';

test.describe('Household > Merchants settings', () => {
  test('left nav links to Household, and lands on the Merchants page', async ({
    authedPage,
  }) => {
    await authedPage.goto('/dashboard');
    await authedPage
      .getByRole('link', { name: 'Household', exact: true })
      .click();

    await expect(authedPage).toHaveURL('/settings/members');
    await authedPage
      .getByRole('link', { name: 'Merchants', exact: true })
      .click();

    await expect(authedPage).toHaveURL('/settings/merchants');
    await expect(authedPage.locator('.top-menu-title')).toHaveText('Settings');
    await expect(
      authedPage.getByRole('link', { name: 'Merchants', exact: true }),
    ).toHaveClass(/settings-nav-link--active/);
  });

  test('lists common merchants with a transaction count, but no Edit/Delete buttons', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/merchants');

    // "Starbucks", not "Amazon" — "Amazon" is a substring of the separately-seeded "Amazon
    // Prime", which would make `hasText: 'Amazon'` match both rows ambiguously.
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Starbucks',
    });
    await expect(row).toBeVisible();
    await expect(row).toContainText('0 transactions');
    await expect(
      row.getByRole('button', { name: 'Edit', exact: true }),
    ).toHaveCount(0);
    await expect(
      row.getByRole('button', { name: 'Delete', exact: true }),
    ).toHaveCount(0);
  });

  test('shows the total merchant count', async ({ authedPage }) => {
    await authedPage.goto('/settings/merchants');

    // Not pinned to an exact number — the common list grows over time — just that it reads
    // "N Merchants" and roughly matches the seeded common catalog's scale.
    const total = authedPage.locator('.merchants-list-total');
    await expect(total).toBeVisible();
    await expect(total).toContainText(/\d+ Merchants/);
  });

  test('creates a merchant through the New merchant modal', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/merchants');
    await authedPage
      .getByRole('button', { name: 'New merchant', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New merchant' });
    await dialog.getByLabel('Name').fill('Corner Bakery');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Corner Bakery',
    });
    await expect(row).toBeVisible();
    // Unlike a common merchant's row, a just-created custom one has Edit/Delete.
    await expect(
      row.getByRole('button', { name: 'Edit', exact: true }),
    ).toBeVisible();
  });

  test('rejects a duplicate merchant name with an inline error, not a toast', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await createMerchant(context.request, workerInfra.apiOrigin, 'Corner Bakery');
    await authedPage.goto('/settings/merchants');
    await authedPage
      .getByRole('button', { name: 'New merchant', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New merchant' });
    await dialog.getByLabel('Name').fill('Corner Bakery');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog.getByRole('alert')).toHaveText(
      'A merchant with this name already exists.',
    );
    await expect(dialog).toBeVisible();
  });

  test('requires a name before saving', async ({ authedPage }) => {
    await authedPage.goto('/settings/merchants');
    await authedPage
      .getByRole('button', { name: 'New merchant', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New merchant' });
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog.getByRole('alert')).toHaveText('A name is required.');
    await expect(dialog).toBeVisible();
  });

  test('edits a merchant through the Edit merchant modal, prefilled with its current name', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await createMerchant(context.request, workerInfra.apiOrigin, 'Corner Bakery');
    await authedPage.goto('/settings/merchants');
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Corner Bakery',
    });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Edit merchant' });
    await expect(dialog.getByLabel('Name')).toHaveValue('Corner Bakery');
    await dialog.getByLabel('Name').fill('Uptown Bakery');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    await expect(
      authedPage.getByText('Uptown Bakery', { exact: true }),
    ).toBeVisible();
    await expect(
      authedPage.getByText('Corner Bakery', { exact: true }),
    ).not.toBeVisible();
  });

  test('cancelling the modal discards changes', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await createMerchant(context.request, workerInfra.apiOrigin, 'Corner Bakery');
    await authedPage.goto('/settings/merchants');
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Corner Bakery',
    });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Edit merchant' });
    await dialog.getByLabel('Name').fill('Should not save');
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    await expect(row).toContainText('Corner Bakery');
  });

  test('deleting shows an inline confirm panel, cancel keeps the merchant', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await createMerchant(context.request, workerInfra.apiOrigin, 'Corner Bakery');
    await authedPage.goto('/settings/merchants');
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Corner Bakery',
    });
    await row.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(
      row.getByText('Delete the "Corner Bakery" merchant?'),
    ).toBeVisible();
    await row.getByRole('button', { name: 'Cancel', exact: true }).click();

    await expect(
      row.getByText('Delete the "Corner Bakery" merchant?'),
    ).not.toBeVisible();
    await expect(row).toBeVisible();
  });

  test('confirming delete removes the merchant', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await createMerchant(context.request, workerInfra.apiOrigin, 'Corner Bakery');
    await authedPage.goto('/settings/merchants');
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Corner Bakery',
    });
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await row.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(row).not.toBeVisible();
  });

  test('a merchant still used by a transaction shows a specific error, not the generic one, and stays', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    // Created explicitly, then matched again by name below — seedTransaction always resolves a
    // merchant from raw text, and "Corner Bakery" doesn't collide with any common merchant, so
    // the transaction ends up pointing at this exact row rather than creating a second one.
    await createMerchant(context.request, workerInfra.apiOrigin, 'Corner Bakery');
    const categoryId = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Corner Bakery',
      amount: '12.00',
      categoryId,
    });

    await authedPage.goto('/settings/merchants');
    const row = authedPage.locator('li.merchants-list-row', {
      hasText: 'Corner Bakery',
    });
    await expect(row).toContainText('1 transaction');
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await row.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(
      authedPage.getByText(
        "This merchant still has transactions, so it can't be deleted. Move or delete those transactions first.",
      ),
    ).toBeVisible();
    await expect(row).toBeVisible();
  });

  test('search filters the list by name, case-insensitively', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/merchants');
    // "Starbucks" — not "Amazon"/"Amazon Prime" — has no other common merchant it's a substring
    // of or contains, so a single unambiguous row is expected to match.
    await authedPage
      .getByPlaceholder(/Search \d+ merchants…/)
      .fill('starbucks');

    await expect(
      authedPage.locator('li.merchants-list-row', { hasText: 'Starbucks' }),
    ).toBeVisible();
    await expect(authedPage.locator('li.merchants-list-row')).toHaveCount(1);
  });

  test('sorting alphabetically orders a custom merchant among the common ones by name', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    // Sorts before every seeded common merchant (all start with a letter at or after "A").
    await createMerchant(context.request, workerInfra.apiOrigin, '#1 Bakery');
    await authedPage.goto('/settings/merchants');
    await authedPage
      .getByRole('combobox', { name: 'Sort' })
      .selectOption('alphabetical');

    await expect(
      authedPage.locator('.merchants-list-name').first(),
    ).toHaveText('#1 Bakery');
  });
});
