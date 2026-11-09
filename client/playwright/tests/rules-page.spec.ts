import { test, expect } from '../fixtures';
import type { APIRequestContext } from '@playwright/test';
import { getCategoryId, seedTransaction } from '../lib/seed';

/** Creates a rule that recategorizes transactions whose statement contains `statement`. */
const createRule = async (
  request: APIRequestContext,
  api: string,
  statement: string,
  categoryId: number,
) => {
  const response = await request.post(`${api}/rules`, {
    data: {
      conditions: {
        original_statement: [{ operator: 'contains', value: statement }],
      },
      actions: { category_id: categoryId },
      apply_to_existing: false,
    },
  });
  if (!response.ok()) throw new Error(`rule not created: ${response.status()}`);
};

test.describe('Household > Rules settings', () => {
  test('creates a rule, previews it, and applies it to existing transactions', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { request } = context;
    const api = workerInfra.apiOrigin;
    const uncategorized = await getCategoryId(request, api, 'Uncategorized');
    await seedTransaction(request, api, {
      date: '2026-11-01',
      merchant: 'SPOTIFY P1234',
      amount: '12.99',
      categoryId: uncategorized,
    });
    await seedTransaction(request, api, {
      date: '2026-11-02',
      merchant: 'NETFLIX.COM',
      amount: '16.99',
      categoryId: uncategorized,
    });

    await authedPage.goto('/settings/rules');
    await authedPage
      .getByRole('button', { name: 'Create rule', exact: true })
      .click();
    const dialog = authedPage.getByRole('dialog', { name: 'New rule' });

    await dialog.getByRole('switch', { name: 'Original statement' }).click();
    await dialog
      .getByRole('textbox', { name: 'Original statement' })
      .fill('spotify');
    await dialog.getByRole('switch', { name: 'Update category' }).click();
    await dialog.getByRole('button', { name: 'Update category' }).click();
    await authedPage.locator('.category-picker-search').fill('Entertainment');
    await authedPage.keyboard.press('Enter');

    await expect(dialog).toContainText('1 matching transaction');
    await dialog.getByRole('tab', { name: /Preview changes/ }).click();
    await expect(dialog.locator('.rule-preview-row')).toHaveCount(1);
    await expect(dialog.locator('.rule-preview-row')).toContainText(
      'Entertainment & Recreation',
    );

    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(authedPage.locator('.rules-list-item')).toHaveCount(1);
    await expect(authedPage.locator('.rules-list-item')).toContainText(
      '1 transaction updated',
    );

    // The statements match the common Spotify and Netflix merchants.
    await authedPage.goto('/transactions');
    await expect(
      authedPage.locator('li.transactions-row', { hasText: /spotify/i }),
    ).toContainText('Entertainment');
    await expect(
      authedPage.locator('li.transactions-row', { hasText: /netflix/i }),
    ).toContainText('Uncategorized');
  });

  test('requires a condition and an update before saving', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/rules');
    await authedPage
      .getByRole('button', { name: 'Create rule', exact: true })
      .click();
    const dialog = authedPage.getByRole('dialog', { name: 'New rule' });

    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog.getByRole('alert')).toHaveText(
      'Turn on at least one condition.',
    );

    await dialog.getByRole('switch', { name: 'Original statement' }).click();
    await dialog
      .getByRole('textbox', { name: 'Original statement' })
      .fill('coffee');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog.getByRole('alert')).toHaveText(
      'Turn on at least one update.',
    );
  });

  test('searches rules by their statements and names', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { request } = context;
    const api = workerInfra.apiOrigin;
    await createRule(
      request,
      api,
      'coffee',
      await getCategoryId(request, api, 'Restaurants & Bars'),
    );
    await createRule(
      request,
      api,
      'spotify',
      await getCategoryId(request, api, 'Entertainment & Recreation'),
    );

    await authedPage.goto('/settings/rules');
    const rules = authedPage.locator('.rules-list-item');
    await expect(rules).toHaveCount(2);

    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();
    await authedPage.getByRole('textbox', { name: 'Search' }).fill('coffee');
    await authedPage.keyboard.press('Enter');
    await expect(rules).toHaveCount(1);
    await expect(rules).toContainText('coffee');

    // A category name is searched too.
    await authedPage.getByRole('button', { name: '"coffee"' }).click();
    await authedPage
      .getByRole('textbox', { name: 'Search' })
      .fill('entertainment');
    await authedPage.keyboard.press('Enter');
    await expect(rules).toHaveCount(1);
    await expect(rules).toContainText('spotify');

    await authedPage.getByRole('button', { name: '"entertainment"' }).click();
    await authedPage
      .getByRole('textbox', { name: 'Search' })
      .fill('nothing like this');
    await authedPage.keyboard.press('Enter');
    await expect(rules).toHaveCount(0);
    await expect(
      authedPage.getByText('No rule matches these filters.'),
    ).toBeVisible();

    await authedPage
      .getByRole('button', { name: '"nothing like this"' })
      .click();
    await authedPage
      .getByRole('button', { name: 'Clear', exact: true })
      .click();
    await expect(rules).toHaveCount(2);
  });

  test('filters rules by category', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { request } = context;
    const api = workerInfra.apiOrigin;
    await createRule(
      request,
      api,
      'coffee',
      await getCategoryId(request, api, 'Restaurants & Bars'),
    );
    await createRule(
      request,
      api,
      'spotify',
      await getCategoryId(request, api, 'Entertainment & Recreation'),
    );

    await authedPage.goto('/settings/rules');
    const rules = authedPage.locator('.rules-list-item');
    await expect(rules).toHaveCount(2);

    await authedPage
      .getByRole('button', { name: 'Filters', exact: true })
      .click();
    const popover = authedPage.getByRole('dialog', { name: 'Filters' });
    await popover
      .getByRole('checkbox', { name: 'Entertainment & Recreation' })
      .check();
    await expect(popover).toContainText('1 filter selected');
    await popover.getByRole('button', { name: 'Apply' }).click();

    await expect(rules).toHaveCount(1);
    await expect(rules).toContainText('spotify');
    await expect(
      authedPage.getByRole('button', { name: 'Filters (1)' }),
    ).toBeVisible();

    // Switching sections shows that section's options.
    await authedPage.getByRole('button', { name: 'Filters (1)' }).click();
    await popover.getByRole('button', { name: 'Tags' }).click();
    await expect(popover.getByRole('checkbox', { name: 'Tax' })).toBeVisible();
    await expect(
      popover.getByRole('checkbox', { name: 'Restaurants & Bars' }),
    ).toHaveCount(0);

    // The footer's Clear (each summary group has its own Clear link too).
    await popover
      .locator('.filters-popover-actions')
      .getByRole('button', { name: 'Clear' })
      .click();
    await expect(rules).toHaveCount(2);
    await expect(
      authedPage.getByRole('button', { name: 'Filters', exact: true }),
    ).toBeVisible();
  });

  test('reorders rules and keeps the new order', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { request } = context;
    const api = workerInfra.apiOrigin;
    const category = await getCategoryId(request, api, 'Restaurants & Bars');
    await createRule(request, api, 'first', category);
    await createRule(request, api, 'second', category);

    await authedPage.goto('/settings/rules');
    const rules = authedPage.locator('.rules-list-item');
    await expect(rules.nth(0)).toContainText('first');

    const saved = authedPage.waitForResponse(
      (response) => response.url().endsWith('/rules/reorder') && response.ok(),
    );
    await authedPage.getByRole('button', { name: 'Move rule 1 down' }).click();
    await expect(rules.nth(0)).toContainText('second');
    await expect(rules.nth(1)).toContainText('first');

    await saved;
    await authedPage.reload();
    await expect(rules.nth(0)).toContainText('second');
    await expect(rules.nth(1)).toContainText('first');
  });

  test('edits and deletes a rule', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { request } = context;
    const api = workerInfra.apiOrigin;
    await createRule(
      request,
      api,
      'coffee',
      await getCategoryId(request, api, 'Restaurants & Bars'),
    );

    await authedPage.goto('/settings/rules');
    await authedPage.getByRole('button', { name: 'Edit rule 1' }).click();
    const dialog = authedPage.getByRole('dialog', { name: 'Edit rule' });
    await expect(
      dialog.getByRole('checkbox', { name: 'Update existing transactions' }),
    ).not.toBeChecked();

    await dialog
      .getByRole('textbox', { name: 'Original statement' })
      .fill('espresso');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    const rules = authedPage.locator('.rules-list-item');
    await expect(rules).toContainText('espresso');
    await expect(rules).not.toContainText('coffee');

    await authedPage.getByRole('button', { name: 'Edit rule 1' }).click();
    await dialog.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(dialog).toContainText(
      'Delete this rule? Transactions it already changed stay as they are.',
    );
    await dialog.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(rules).toHaveCount(0);
  });
});
