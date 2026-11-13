import type { APIRequestContext } from '@playwright/test';
import { test, expect } from '../fixtures';
import { ensureAccount, getCategoryId } from '../lib/seed';

/** Adds a transaction, waiting on Anyone for a review when `needsReview`. Returns its id. */
const addTransaction = async (
  request: APIRequestContext,
  apiOrigin: string,
  {
    date,
    merchant,
    amount = '20.00',
    needsReview = false,
  }: { date: string; merchant: string; amount?: string; needsReview?: boolean },
): Promise<number> => {
  const response = await request.post(`${apiOrigin}/transactions`, {
    data: {
      date,
      original_statement: merchant,
      amount,
      category_id: await getCategoryId(request, apiOrigin, 'Groceries'),
      account_id: await ensureAccount(request, apiOrigin, 'Visa'),
    },
  });
  expect(response.ok()).toBe(true);
  const id = ((await response.json()) as { id: number }).id;
  if (needsReview) {
    const patched = await request.patch(`${apiOrigin}/transactions/${id}`, {
      data: { review_assignee: 'anyone' },
    });
    expect(patched.ok()).toBe(true);
  }
  return id;
};

test.describe('Dashboard Transactions card', () => {
  test('lists the five most recent transactions and links to the Transactions page', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    const merchants = [
      'Oldest Bakery',
      'Corner Deli',
      'Pet Shop',
      'Book Nook',
      'Bike Repair',
      'Newest Florist',
    ];
    for (const [index, merchant] of merchants.entries()) {
      await addTransaction(context.request, apiOrigin, {
        date: `2026-10-${String(10 + index).padStart(2, '0')}`,
        merchant,
      });
    }

    await authedPage.goto('/dashboard');
    const card = authedPage.locator('.recent-transactions-card');
    const rows = card.locator('li.transactions-row');
    await expect(rows).toHaveCount(5);
    await expect(rows.first()).toContainText('Newest Florist');
    await expect(card).not.toContainText('Oldest Bakery');
    await expect(card).toContainText('Most recent');

    await card.getByRole('link', { name: 'Transactions', exact: true }).click();
    await expect(authedPage).toHaveURL('/transactions');
  });

  test('edits a row in place and opens it over the dashboard', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    const id = await addTransaction(context.request, apiOrigin, {
      date: '2026-10-20',
      merchant: 'Corner Deli',
    });

    await authedPage.goto('/dashboard');
    const row = authedPage
      .locator('.recent-transactions-card li.transactions-row')
      .first();
    await row.getByRole('button', { name: '$20.00' }).click();
    const amount = row.getByRole('textbox', { name: 'Amount' });
    await amount.fill('42.50');
    await amount.press('Enter');
    await expect(row.getByRole('button', { name: '$42.50' })).toBeVisible();
    await expect
      .poll(
        async () =>
          (
            await (
              await context.request.get(`${apiOrigin}/transactions/${id}`)
            ).json()
          ).amount,
      )
      .toBe('42.50');

    await row.getByRole('button', { name: 'Edit transaction' }).click();
    const dialog = authedPage.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(authedPage).toHaveURL('/dashboard');
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).not.toBeVisible();
  });

  test('the review view narrows the card and carries over to the Transactions page', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const { apiOrigin } = workerInfra;
    await addTransaction(context.request, apiOrigin, {
      date: '2026-10-21',
      merchant: 'Corner Deli',
    });
    await addTransaction(context.request, apiOrigin, {
      date: '2026-10-20',
      merchant: 'Pet Shop',
      needsReview: true,
    });

    await authedPage.goto('/dashboard');
    const card = authedPage.locator('.recent-transactions-card');
    const row = (merchant: string) =>
      card.locator('li.transactions-row', { hasText: merchant });
    await expect(row('Corner Deli')).toBeVisible();

    await card.getByRole('button', { name: /^All transactions/ }).click();
    await authedPage
      .locator('.review-view-popover')
      .getByRole('button', { name: /^Needs review\s*1$/ })
      .click();
    await expect(row('Pet Shop')).toBeVisible();
    await expect(row('Corner Deli')).not.toBeVisible();

    await row('Pet Shop')
      .getByRole('button', { name: 'Mark Pet Shop reviewed', exact: true })
      .click();
    await expect(card.locator('.review-empty')).toBeVisible();

    await card.getByRole('button', { name: 'View all transactions' }).click();
    await expect(authedPage).toHaveURL('/transactions');

    // The title opens the Transactions page in the card's review view.
    await authedPage.goto('/dashboard');
    await card.getByRole('button', { name: /^All transactions/ }).click();
    await authedPage
      .locator('.review-view-popover')
      .getByRole('button', { name: /^Needs review/ })
      .click();
    await card.getByRole('link', { name: 'Transactions', exact: true }).click();
    await expect(authedPage).toHaveURL('/transactions?needs_review=all');
  });
});

test.describe('Customize dashboard', () => {
  test('the move grip shows on hover without moving the title', async ({
    authedPage,
  }) => {
    await authedPage.goto('/dashboard');
    const widget = authedPage.locator('.dashboard-widget').first();
    const grip = widget.locator('.widget-move-handle');
    const title = widget.getByRole('link', {
      name: 'Transactions',
      exact: true,
    });
    await expect(grip).toHaveCSS('opacity', '0');
    const before = await title.boundingBox();

    await widget.hover();
    await expect(grip).toHaveCSS('opacity', '1');
    expect((await title.boundingBox())?.x).toBe(before?.x);
    // The header sits right on top of the list, with no gap between them.
    const header = await widget.locator('header').boundingBox();
    const body = await widget.locator('.transactions-card-body').boundingBox();
    expect(body?.y).toBe((header?.y ?? 0) + (header?.height ?? 0));
    // Moving with the keyboard is the Customize window's job.
    await expect(widget.getByRole('button', { name: /^Move / })).toHaveCount(0);
  });

  test('hides and shows the Transactions widget, and remembers it', async ({
    authedPage,
  }) => {
    await authedPage.goto('/dashboard');
    const card = authedPage.locator('.recent-transactions-card');
    await expect(card).toBeVisible();

    await authedPage.getByRole('button', { name: 'Customize' }).click();
    const dialog = authedPage.getByRole('dialog', {
      name: 'Customize dashboard',
    });
    await expect(dialog).toBeVisible();
    const toggle = dialog.getByRole('switch', { name: 'Show Transactions' });
    await expect(toggle).toHaveAttribute('aria-checked', 'true');

    // With a single widget, there's nowhere to move it.
    await expect(
      dialog.getByRole('button', { name: 'Move Transactions up' }),
    ).toBeDisabled();
    await expect(
      dialog.getByRole('button', { name: 'Move Transactions down' }),
    ).toBeDisabled();

    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-checked', 'false');
    await expect(card).not.toBeVisible();
    await expect(authedPage.getByText(/Every widget is hidden/)).toBeVisible();

    await authedPage.keyboard.press('Escape');
    await expect(dialog).not.toBeVisible();
    await expect(
      authedPage.getByRole('button', { name: 'Customize' }),
    ).toBeFocused();

    await authedPage.reload();
    await expect(authedPage.getByText(/Every widget is hidden/)).toBeVisible();
    await expect(card).not.toBeVisible();

    await authedPage.getByRole('button', { name: 'Customize' }).click();
    await dialog.getByRole('switch', { name: 'Show Transactions' }).click();
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(card).toBeVisible();
  });

  test('keeps the widget shown, with a toast, when the save fails', async ({
    authedPage,
    workerInfra,
  }) => {
    await authedPage.route(`${workerInfra.apiOrigin}/settings`, (route) =>
      route.request().method() === 'PATCH'
        ? route.fulfill({ status: 500, body: '{}' })
        : route.continue(),
    );
    await authedPage.goto('/dashboard');
    await authedPage.getByRole('button', { name: 'Customize' }).click();
    const toggle = authedPage
      .getByRole('dialog', { name: 'Customize dashboard' })
      .getByRole('switch', { name: 'Show Transactions' });

    await toggle.click();
    await expect(
      authedPage.getByText('Failed to save your settings. Please try again.'),
    ).toBeVisible();
    await expect(toggle).toHaveAttribute('aria-checked', 'true');
    await expect(authedPage.locator('.recent-transactions-card')).toBeVisible();
  });

  test('closes on an outside click', async ({ authedPage }) => {
    await authedPage.goto('/dashboard');
    await authedPage.getByRole('button', { name: 'Customize' }).click();
    const dialog = authedPage.getByRole('dialog', {
      name: 'Customize dashboard',
    });
    await expect(dialog).toBeVisible();
    await authedPage.mouse.click(5, 5);
    await expect(dialog).not.toBeVisible();
  });
});
