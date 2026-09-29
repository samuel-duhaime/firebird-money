import type { APIRequestContext, Page } from '@playwright/test';
import { test, expect } from '../fixtures';
import { getCategoryId, seedTransaction } from '../lib/seed';
import {
  DATE_RANGE_PRESETS,
  resolvePreset,
} from '../../src/features/transactions/utils/date-range';
import type { DateRangePreset } from '../../src/features/transactions/utils/date-range';
import {
  formatAmount,
  formatDateHeading,
} from '../../src/features/transactions/utils/format';

// TODO: The import-budget-file flow (ImportButton.tsx, POST /transactions/import) isn't covered
// here — it shells out to a `claude` CLI subprocess server-side (see
// server/src/features/transactions/import.rs and the README's "Install" section), which isn't
// available in this test environment (or CI) and isn't something these tests can/should
// provision. Revisit once import has a testable seam.

const LOCALE = 'en-US'; // the app's default language until something sets `firebird-language`
const money = (amount: number) => formatAmount(amount, LOCALE);
const dateHeading = (isoDate: string) => formatDateHeading(isoDate, LOCALE);

const PRESET_LABELS: Record<DateRangePreset, string> = {
  last_7_days: 'Last 7 days',
  last_30_days: 'Last 30 days',
  this_month: 'This month',
  last_month: 'Last month',
  this_year: 'This year',
  last_year: 'Last year',
};

/** Shifts a `YYYY-MM-DD` key by `days` (may be negative), for building dates safely outside a
 * resolved preset range without depending on the preset's own internal day-math. */
const shiftDateKey = (dateKey: string, days: number): string => {
  const [year, month, day] = dateKey.split('-').map(Number);
  const shifted = new Date(year, month - 1, day + days);
  return `${shifted.getFullYear()}-${String(shifted.getMonth() + 1).padStart(2, '0')}-${String(shifted.getDate()).padStart(2, '0')}`;
};

/** The add-transaction form's category field is the same searchable `CategoryPicker` popover as
 * the transactions-list inline edit, portaled to `document.body` rather than nested in the
 * dialog — so it's queried against `page`, not the dialog locator. */
const selectCategory = async (
  page: Page,
  triggerName: string,
  optionName: string,
): Promise<void> => {
  await page.getByRole('button', { name: triggerName, exact: true }).click();
  await page
    .locator('.category-picker-popover')
    .getByRole('button', { name: optionName, exact: true })
    .click();
};

test.describe('list, grouping, and daily subtotal', () => {
  test('shows an empty state with no transactions', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await expect(authedPage.getByText('No transactions yet.')).toBeVisible();
  });

  test('shows a loading state while the request is in flight', async ({
    authedPage,
    workerInfra,
  }) => {
    // Scoped to the API origin, not just "**/transactions" — that glob also matches the browser's
    // own navigation request to the client origin's /transactions route, which would otherwise
    // get delayed/replaced too.
    await authedPage.route(
      `${workerInfra.apiOrigin}/transactions`,
      async (route) => {
        await new Promise((resolve) => setTimeout(resolve, 500));
        await route.continue();
      },
    );

    await authedPage.goto('/transactions');
    await expect(authedPage.getByText('Loading transactions…')).toBeVisible();
    await expect(authedPage.getByText('No transactions yet.')).toBeVisible();
  });

  test('shows an error state when the request fails', async ({
    authedPage,
    workerInfra,
  }) => {
    // Scoped to the API origin — see the loading-state test above for why "**/transactions"
    // isn't safe to use for a GET-only mock.
    await authedPage.route(
      `${workerInfra.apiOrigin}/transactions`,
      async (route) => {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: '{}',
        });
      },
    );

    await authedPage.goto('/transactions');
    // React Query retries a failed query a few times with backoff before settling into the
    // error state, so this needs more headroom than the default assertion timeout.
    await expect(
      authedPage.getByText('Failed to load transactions.'),
    ).toBeVisible({ timeout: 15_000 });
  });

  test('groups same-day transactions, subtotals only expenses, and marks credits', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const salary = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Paychecks',
    );

    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-02-10',
      merchant: 'Grocery Run',
      amount: '45.00',
      categoryId: groceries,
    });
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-02-10',
      merchant: 'Paycheck',
      amount: '1000.00',
      categoryId: salary,
    });
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-02-09',
      merchant: 'Coffee Shop',
      amount: '5.00',
      categoryId: groceries,
    });

    await authedPage.goto('/transactions');

    const headers = authedPage.locator('.transactions-date-header');
    await expect(headers).toHaveCount(2);
    // Default sort is newest-first.
    await expect(headers.nth(0)).toContainText(dateHeading('2023-02-10'));
    await expect(headers.nth(0)).toContainText(money(45)); // income excluded from the subtotal
    await expect(headers.nth(1)).toContainText(dateHeading('2023-02-09'));
    await expect(headers.nth(1)).toContainText(money(5));

    const paycheckRow = authedPage.locator('li.transactions-row', {
      hasText: 'Paycheck',
    });
    await expect(paycheckRow.locator('.transactions-row-amount')).toContainText(
      '+',
    );
    const groceryRow = authedPage.locator('li.transactions-row', {
      hasText: 'Grocery Run',
    });
    await expect(
      groceryRow.locator('.transactions-row-amount'),
    ).not.toContainText('+');
  });
});

test.describe('add transaction', () => {
  test('opens at /transactions/add-transaction', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    await expect(
      authedPage.getByRole('dialog', { name: 'Add transaction' }),
    ).toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions\/add-transaction$/);
  });

  test('closes via Escape, the close button, and Cancel — each returning to /transactions', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const addButton = authedPage.getByRole('button', {
      name: 'Add',
      exact: true,
    });
    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });

    await addButton.click();
    await expect(dialog).toBeVisible();
    await authedPage.keyboard.press('Escape');
    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);

    await addButton.click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);

    await addButton.click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);
  });

  test('closes on an outside click, returning to /transactions', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    await expect(dialog).toBeVisible();

    // The overlay covers the whole viewport; a corner is safely outside the centered dialog box.
    await authedPage
      .locator('.modal-overlay')
      .click({ position: { x: 5, y: 5 } });

    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);
  });

  test('autofocuses on open, traps Tab within the dialog, and returns focus to Add on close', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const addButton = authedPage.getByRole('button', {
      name: 'Add',
      exact: true,
    });
    await addButton.click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    const closeButton = dialog.getByRole('button', {
      name: 'Close',
      exact: true,
    });
    const submitButton = dialog.getByRole('button', {
      name: 'Add transaction',
      exact: true,
    });
    const amountInput = dialog.getByLabel('Amount');

    // The amount field — not the × close button — is what gets focused on open.
    await expect(amountInput).toBeFocused();

    // Shift+Tab from the amount field moves back to the close button, its previous sibling in
    // tab order.
    await authedPage.keyboard.press('Shift+Tab');
    await expect(closeButton).toBeFocused();

    // Shift+Tab again from the close button — now the first element — wraps around to the last.
    await authedPage.keyboard.press('Shift+Tab');
    await expect(submitButton).toBeFocused();

    // Tab from the last element wraps back around to the first.
    await authedPage.keyboard.press('Tab');
    await expect(closeButton).toBeFocused();

    await closeButton.click();
    await expect(dialog).not.toBeVisible();
    await expect(addButton).toBeFocused();
  });

  test('adds a transaction through the modal', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    await dialog.getByLabel('Amount').fill('12.50');
    await dialog.getByLabel('Merchant').fill('Corner Store');
    await dialog.getByLabel('Date').fill('2023-03-01');
    await selectCategory(authedPage, 'Select category', 'Groceries');
    await dialog
      .getByRole('button', { name: 'Add transaction', exact: true })
      .click();

    await expect(dialog).not.toBeVisible();
    await expect(authedPage.getByText('Transaction added.')).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Corner Store' }),
    ).toBeVisible();
  });

  test('requires every field', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    await dialog
      .getByRole('button', { name: 'Add transaction', exact: true })
      .click();

    await expect(dialog.getByRole('alert')).toHaveText(
      'All fields are required.',
    );
    await expect(dialog).toBeVisible();
  });

  test('shows an inline error, not a toast, when the create request fails', async ({
    authedPage,
  }) => {
    await authedPage.route('**/transactions', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: '{}',
        });
      } else {
        await route.continue();
      }
    });

    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    await dialog.getByLabel('Amount').fill('12.50');
    await dialog.getByLabel('Merchant').fill('Corner Store');
    await dialog.getByLabel('Date').fill('2023-03-01');
    await selectCategory(authedPage, 'Select category', 'Groceries');
    await dialog
      .getByRole('button', { name: 'Add transaction', exact: true })
      .click();

    await expect(dialog.getByRole('alert')).toHaveText(
      'Failed to add the transaction. Please try again.',
    );
    await expect(dialog).toBeVisible();
    await expect(authedPage.getByText('Transaction added.')).not.toBeVisible();
  });

  test.describe('amount normalization', () => {
    const fillAndSubmit = async (
      page: import('@playwright/test').Page,
      rawAmount: string,
    ) => {
      await page.goto('/transactions');
      await page.getByRole('button', { name: 'Add', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Add transaction' });
      await dialog.getByLabel('Amount').fill(rawAmount);
      await dialog.getByLabel('Merchant').fill('Normalization Check');
      await dialog.getByLabel('Date').fill('2023-03-01');
      await selectCategory(page, 'Select category', 'Groceries');
      await dialog
        .getByRole('button', { name: 'Add transaction', exact: true })
        .click();
      return dialog;
    };

    test('accepts a period decimal separator', async ({ authedPage }) => {
      const dialog = await fillAndSubmit(authedPage, '12.50');
      await expect(dialog).not.toBeVisible();
      await expect(
        authedPage.locator('li.transactions-row', {
          hasText: 'Normalization Check',
        }),
      ).toBeVisible();
    });

    test('accepts and normalizes a comma decimal separator', async ({
      authedPage,
    }) => {
      const dialog = await fillAndSubmit(authedPage, '12,50');
      await expect(dialog).not.toBeVisible();
      await expect(
        authedPage.locator('li.transactions-row', {
          hasText: 'Normalization Check',
        }),
      ).toBeVisible();
    });

    test('rejects a value that reads as thousands-grouped', async ({
      authedPage,
    }) => {
      const dialog = await fillAndSubmit(authedPage, '1,234');
      await expect(dialog.getByRole('alert')).toHaveText(
        'Enter a plain amount, e.g. 12.50, without thousands separators.',
      );
      await expect(dialog).toBeVisible();
    });

    test('rejects a value with two separators', async ({ authedPage }) => {
      const dialog = await fillAndSubmit(authedPage, '1,234.56');
      await expect(dialog.getByRole('alert')).toHaveText(
        'Enter a plain amount, e.g. 12.50, without thousands separators.',
      );
      await expect(dialog).toBeVisible();
    });

    test('rejects a whole-number part longer than the server can store (NUMERIC(12,2))', async ({
      authedPage,
    }) => {
      const dialog = await fillAndSubmit(authedPage, '12345678901.50');
      await expect(dialog.getByRole('alert')).toHaveText(
        'Enter an amount with at most 10 digits before the decimal point.',
      );
      await expect(dialog).toBeVisible();
    });

    test('strips non-numeric characters as they are typed', async ({
      authedPage,
    }) => {
      await authedPage.goto('/transactions');
      await authedPage
        .getByRole('button', { name: 'Add', exact: true })
        .click();
      const dialog = authedPage.getByRole('dialog', {
        name: 'Add transaction',
      });
      await dialog.getByLabel('Amount').fill('12.50abc$');
      await expect(dialog.getByLabel('Amount')).toHaveValue('12.50');
    });

    test('strips a minus sign as it is typed, so amounts can never go negative', async ({
      authedPage,
    }) => {
      await authedPage.goto('/transactions');
      await authedPage
        .getByRole('button', { name: 'Add', exact: true })
        .click();
      const dialog = authedPage.getByRole('dialog', {
        name: 'Add transaction',
      });
      await dialog.getByLabel('Amount').fill('-12.50');
      await expect(dialog.getByLabel('Amount')).toHaveValue('12.50');
    });
  });

  test('uses the same searchable, grouped category popover as the transactions list', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    await authedPage
      .getByRole('button', { name: 'Select category', exact: true })
      .click();
    const popover = authedPage.locator('.category-picker-popover');
    await expect(popover).toBeVisible();
    await expect(popover.getByText('Food & Dining')).toBeVisible();
    await popover.getByPlaceholder('Search categories...').fill('groceries');
    await popover
      .getByRole('button', { name: 'Groceries', exact: true })
      .click();

    await expect(
      authedPage.getByRole('button', { name: 'Groceries', exact: true }),
    ).toBeVisible();
  });
});

test.describe('edit and delete transaction', () => {
  const seedCornerStore = async (
    request: APIRequestContext,
    apiOrigin: string,
    categoryName: 'Groceries' | 'Coffee Shops' = 'Groceries',
  ): Promise<void> => {
    const categoryId = await getCategoryId(request, apiOrigin, categoryName);
    await seedTransaction(request, apiOrigin, {
      date: '2023-04-01',
      merchant: 'Corner Store',
      amount: '20.00',
      categoryId,
    });
  };

  const editTriggerButton = (authedPage: Page) =>
    authedPage
      .locator('li.transactions-row', { hasText: 'Corner Store' })
      .getByRole('button', { name: 'Edit transaction', exact: true });

  const openEditPanel = async (authedPage: Page) => {
    await authedPage.goto('/transactions');
    await editTriggerButton(authedPage).click();
    return authedPage.getByRole('dialog', { name: 'Edit transaction' });
  };

  test('opens via the row, updates the URL to /transactions/:id, and prefills every field', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(dialog).toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions\/\d+$/);
    await expect(dialog.getByLabel('Amount')).toHaveValue('20.00');
    await expect(dialog.getByLabel('Merchant')).toHaveValue('Corner Store');
    await expect(dialog.getByLabel('Date')).toHaveValue('2023-04-01');
    await expect(
      dialog.getByRole('button', { name: 'Groceries', exact: true }),
    ).toBeVisible();
  });

  test('keeps the top menu title and actions visible while the panel is open', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await openEditPanel(authedPage);

    await expect(authedPage.locator('.top-menu-title')).toHaveText(
      'Transactions',
    );
    await expect(
      authedPage.getByRole('button', { name: 'Add', exact: true }),
    ).toBeVisible();
  });

  test('has no Save button — fields commit immediately', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(
      dialog.getByRole('button', { name: 'Save changes', exact: true }),
    ).toHaveCount(0);
  });

  test('autofocuses the amount field on open', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(dialog.getByLabel('Amount')).toBeFocused();
  });

  test('autosaves the merchant on blur', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Merchant').fill('Uptown Store');
    // Clicking another field blurs the merchant input without pressing Enter.
    await dialog.getByLabel('Date').click();

    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Uptown Store' }),
    ).toBeVisible();
  });

  test('autosaves the merchant on Enter', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Merchant').fill('Uptown Store');
    await dialog.getByLabel('Merchant').press('Enter');

    await expect(dialog).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Uptown Store' }),
    ).toBeVisible();
  });

  test('reverts the merchant on Escape, without closing the panel or saving', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Merchant').fill('Should Not Save');
    await authedPage.keyboard.press('Escape');

    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel('Merchant')).toHaveValue('Corner Store');
  });

  test('requires a non-empty merchant, reverting and showing a toast', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Merchant').fill('');
    await dialog.getByLabel('Date').click();

    await expect(authedPage.getByText('All fields are required.')).toBeVisible();
    await expect(dialog.getByLabel('Merchant')).toHaveValue('Corner Store');
  });

  test('autosaves the amount on blur, normalizing a comma decimal separator', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Amount').fill('35,50');
    await dialog.getByLabel('Merchant').click();

    await expect(dialog.getByLabel('Amount')).toHaveValue('35.50');
    await expect(
      authedPage
        .locator('li.transactions-row', { hasText: 'Corner Store' })
        .locator('.transactions-row-amount'),
    ).toContainText(money(35.5));
  });

  test('reverts the amount on Escape, without closing the panel or saving', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Amount').fill('999.99');
    await authedPage.keyboard.press('Escape');

    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel('Amount')).toHaveValue('20.00');
  });

  test('rejects a value that reads as thousands-grouped, reverting and showing a toast', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Amount').fill('1,234');
    await dialog.getByLabel('Merchant').click();

    await expect(
      authedPage.getByText(
        'Enter a plain amount, e.g. 12.50, without thousands separators.',
      ),
    ).toBeVisible();
    await expect(dialog.getByLabel('Amount')).toHaveValue('20.00');
  });

  test('autosaves the date immediately on change', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog.getByLabel('Date').fill('2023-05-15');

    await expect(authedPage.locator('.transactions-date-header')).toContainText(
      dateHeading('2023-05-15'),
    );
  });

  test('autosaves the category immediately on selection', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin, 'Groceries');
    const dialog = await openEditPanel(authedPage);

    await dialog.getByRole('button', { name: 'Groceries', exact: true }).click();
    await authedPage
      .locator('.category-picker-popover')
      .getByRole('button', { name: 'Coffee Shops', exact: true })
      .click();

    await expect(
      dialog.getByRole('button', { name: 'Coffee Shops', exact: true }),
    ).toBeVisible();
    await expect(
      authedPage
        .locator('li.transactions-row', { hasText: 'Corner Store' })
        .getByRole('button', { name: 'Coffee Shops', exact: true }),
    ).toBeVisible();
  });

  test('preserves an in-progress edit in one field while saving another triggers a refetch', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    // Saving the amount invalidates the transaction query, which refetches it — delaying that GET
    // opens a window to start editing the merchant field before the refetch's response would
    // otherwise land mid-keystroke and overwrite it with the pre-edit server value.
    await authedPage.route('**/transactions/*', async (route) => {
      if (route.request().method() === 'GET') {
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
      await route.continue();
    });

    await dialog.getByLabel('Amount').fill('35.50');
    // Focusing the merchant field blurs the amount field, saving it and kicking off the
    // (delayed) refetch above.
    await dialog.getByLabel('Merchant').fill('Uptown Store');

    // Give the delayed refetch time to resolve and re-render while merchant is still focused,
    // unsaved — the exact window where a naive hydration effect would stomp the typed value back
    // to the pre-edit server value.
    await authedPage.waitForTimeout(700);
    await expect(dialog.getByLabel('Merchant')).toHaveValue('Uptown Store');

    await dialog.getByLabel('Date').click();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Uptown Store' }),
    ).toBeVisible();
  });

  test('shows a toast and keeps the old value when an autosave fails', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await authedPage.route('**/transactions/*', async (route) => {
      if (route.request().method() === 'PATCH') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: '{}',
        });
      } else {
        await route.continue();
      }
    });

    await dialog.getByLabel('Merchant').fill('Uptown Store');
    await dialog.getByLabel('Date').click();

    await expect(
      authedPage.getByText(
        'Failed to update the transaction. Please try again.',
      ),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Corner Store' }),
    ).toBeVisible();
  });

  test('closes via the close button, returning to /transactions and focus to the row', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);
    await expect(editTriggerButton(authedPage)).toBeFocused();
  });

  test('closes on Escape when focus is not inside a field that reverts on Escape, returning to /transactions and focus to the row', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(dialog).toBeVisible();
    // The category trigger has no Escape handler of its own, so Escape bubbles up to the panel
    // and closes it — unlike from the amount/merchant fields (see the per-field Escape tests).
    await dialog.getByRole('button', { name: 'Groceries', exact: true }).focus();
    await authedPage.keyboard.press('Escape');

    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);
    await expect(editTriggerButton(authedPage)).toBeFocused();
  });

  test('closes on an outside click, returning to /transactions and focus to the row', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(dialog).toBeVisible();
    // The overlay covers the whole viewport; a corner is safely outside the panel itself, which
    // is anchored to the right edge.
    await authedPage
      .locator('.edit-transaction-overlay')
      .click({ position: { x: 5, y: 5 } });

    await expect(dialog).not.toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);
    await expect(editTriggerButton(authedPage)).toBeFocused();
  });

  test('cancelling delete keeps the transaction and restores the normal footer', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog
      .getByRole('button', { name: 'Delete transaction', exact: true })
      .click();
    await expect(
      dialog.getByText("Delete this transaction? This can't be undone."),
    ).toBeVisible();

    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();

    await expect(
      dialog.getByRole('button', { name: 'Delete transaction', exact: true }),
    ).toBeVisible();
    await expect(dialog).toBeVisible();
  });

  test('deletes the transaction, shows a toast, removes the row, and returns to /transactions', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await dialog
      .getByRole('button', { name: 'Delete transaction', exact: true })
      .click();
    await dialog.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    await expect(authedPage.getByText('Transaction deleted.')).toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions$/);
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Corner Store' }),
    ).not.toBeVisible();
  });

  test('shows a toast and keeps the transaction when delete fails', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await authedPage.route('**/transactions/*', async (route) => {
      if (route.request().method() === 'DELETE') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: '{}',
        });
      } else {
        await route.continue();
      }
    });

    await dialog
      .getByRole('button', { name: 'Delete transaction', exact: true })
      .click();
    await dialog.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(
      authedPage.getByText(
        'Failed to delete the transaction. Please try again.',
      ),
    ).toBeVisible();
    await expect(dialog).toBeVisible();
    await expect(
      dialog.getByRole('button', { name: 'Delete transaction', exact: true }),
    ).toBeVisible();
  });

  test('shows "Transaction not found." for a nonexistent id, without hanging on the loading state', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions/999999999');

    const dialog = authedPage.getByRole('dialog', { name: 'Edit transaction' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText('Transaction not found.')).toBeVisible();
    await expect(
      dialog.getByText('Loading transaction…'),
    ).not.toBeVisible();
  });
});

test.describe('search', () => {
  test('applies a search by pressing Enter in the input', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Whole Foods Market',
      amount: '30.00',
      categoryId: groceries,
    });
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Netflix Subscription',
      amount: '15.00',
      categoryId: groceries,
    });

    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();
    await authedPage.getByPlaceholder('Enter a search term...').fill('foods');
    await authedPage.getByPlaceholder('Enter a search term...').press('Enter');

    await expect(
      authedPage.locator('li.transactions-row', {
        hasText: 'Whole Foods Market',
      }),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', {
        hasText: 'Netflix Subscription',
      }),
    ).not.toBeVisible();
    expect(authedPage.url()).toContain('search=foods');
  });

  test('closes the popover on Escape and on an outside click', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const popover = authedPage.locator('.search-popover');

    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();
    await expect(popover).toBeVisible();
    await authedPage.keyboard.press('Escape');
    await expect(popover).not.toBeVisible();

    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();
    await expect(popover).toBeVisible();
    await authedPage.locator('.top-menu-title').click();
    await expect(popover).not.toBeVisible();
  });

  test('filters the list to a matching merchant, case-insensitively', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Whole Foods Market',
      amount: '30.00',
      categoryId: groceries,
    });
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Netflix Subscription',
      amount: '15.00',
      categoryId: groceries,
    });

    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();
    await authedPage.getByPlaceholder('Enter a search term...').fill('foods');
    await authedPage
      .locator('.search-popover')
      .getByRole('button', { name: 'Apply', exact: true })
      .click();

    await expect(
      authedPage.locator('li.transactions-row', {
        hasText: 'Whole Foods Market',
      }),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', {
        hasText: 'Netflix Subscription',
      }),
    ).not.toBeVisible();
    await expect(authedPage.locator('.top-menu-clear-all')).toBeVisible();
    expect(authedPage.url()).toContain('search=foods');
  });

  test('disables Apply and Clear when there is nothing to apply or clear', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();

    const popover = authedPage.locator('.search-popover');
    await expect(
      popover.getByRole('button', { name: 'Apply', exact: true }),
    ).toBeDisabled();
    await expect(
      popover.getByRole('button', { name: 'Clear', exact: true }),
    ).toBeDisabled();
  });

  test('clears an active search from the popover', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Whole Foods Market',
      amount: '30.00',
      categoryId: groceries,
    });

    await authedPage.goto('/transactions?search=foods');
    await authedPage
      .getByRole('button', { name: '"foods"', exact: true })
      .click();
    await authedPage
      .locator('.search-popover')
      .getByRole('button', { name: 'Clear', exact: true })
      .click();

    await expect(authedPage.locator('.top-menu-clear-all')).not.toBeVisible();
    expect(authedPage.url()).not.toContain('search=');
  });
});

test.describe('date range', () => {
  test('closes the popover on Escape and returns focus to the trigger', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const trigger = authedPage.getByRole('button', {
      name: 'Date',
      exact: true,
    });
    const popover = authedPage.locator('.date-range-popover');

    await trigger.click();
    await expect(popover).toBeVisible();
    await authedPage.keyboard.press('Escape');
    await expect(popover).not.toBeVisible();
    await expect(trigger).toBeFocused();
  });

  test('closes the popover on an outside click and returns focus to the trigger', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const trigger = authedPage.getByRole('button', {
      name: 'Date',
      exact: true,
    });
    const popover = authedPage.locator('.date-range-popover');

    await trigger.click();
    await expect(popover).toBeVisible();
    await authedPage.locator('.top-menu-title').click();
    await expect(popover).not.toBeVisible();
    await expect(trigger).toBeFocused();
  });

  for (const preset of DATE_RANGE_PRESETS) {
    test(`applies the "${PRESET_LABELS[preset]}" preset`, async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      const groceries = await getCategoryId(
        context.request,
        workerInfra.apiOrigin,
        'Groceries',
      );
      const { start_date: inRangeDate } = resolvePreset(preset);
      const outOfRangeDate = shiftDateKey(inRangeDate, -3);

      await seedTransaction(context.request, workerInfra.apiOrigin, {
        date: inRangeDate,
        merchant: 'In Range Merchant',
        amount: '10.00',
        categoryId: groceries,
      });
      await seedTransaction(context.request, workerInfra.apiOrigin, {
        date: outOfRangeDate,
        merchant: 'Out Of Range Merchant',
        amount: '10.00',
        categoryId: groceries,
      });

      await authedPage.goto('/transactions');
      await authedPage
        .getByRole('button', { name: 'Date', exact: true })
        .click();
      await authedPage
        .getByRole('button', { name: PRESET_LABELS[preset], exact: true })
        .click();

      await expect(
        authedPage.locator('li.transactions-row', {
          hasText: 'In Range Merchant',
        }),
      ).toBeVisible();
      await expect(
        authedPage.locator('li.transactions-row', {
          hasText: 'Out Of Range Merchant',
        }),
      ).not.toBeVisible();
    });
  }

  test('filters by a valid manual range', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-01-15',
      merchant: 'In Range Merchant',
      amount: '10.00',
      categoryId: groceries,
    });
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-01-25',
      merchant: 'Out Of Range Merchant',
      amount: '10.00',
      categoryId: groceries,
    });

    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Date', exact: true }).click();
    await authedPage
      .getByLabel('Start date', { exact: true })
      .fill('2023-01-10');
    await authedPage.getByLabel('End date', { exact: true }).fill('2023-01-20');
    await authedPage
      .getByRole('button', { name: 'Apply', exact: true })
      .click();

    await expect(
      authedPage.locator('li.transactions-row', {
        hasText: 'In Range Merchant',
      }),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', {
        hasText: 'Out Of Range Merchant',
      }),
    ).not.toBeVisible();
  });

  test('disables Apply and marks the field invalid for a malformed date', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Date', exact: true }).click();
    await authedPage
      .getByLabel('Start date', { exact: true })
      .fill('not-a-date');

    await expect(
      authedPage.getByLabel('Start date', { exact: true }),
    ).toHaveAttribute('aria-invalid', 'true');
    await expect(
      authedPage.getByRole('button', { name: 'Apply', exact: true }),
    ).toBeDisabled();
  });

  test('disables Apply when start is after end, and re-enables once fixed', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Date', exact: true }).click();
    await authedPage
      .getByLabel('Start date', { exact: true })
      .fill('2023-05-20');
    await authedPage.getByLabel('End date', { exact: true }).fill('2023-05-10');

    await expect(
      authedPage.getByRole('button', { name: 'Apply', exact: true }),
    ).toBeDisabled();

    await authedPage.getByLabel('End date', { exact: true }).fill('2023-05-25');
    await expect(
      authedPage.getByRole('button', { name: 'Apply', exact: true }),
    ).toBeEnabled();
  });

  test('clears an active manual range', async ({ authedPage }) => {
    await authedPage.goto(
      '/transactions?start_date=2023-01-10&end_date=2023-01-20',
    );
    // The trigger's label formats out the year (e.g. "Jan 10 – Jan 20"), so match on the
    // container rather than the label text.
    await authedPage.locator('.date-range-button-trigger button').click();
    await authedPage.locator('.date-range-popover-clear-all').click();

    await expect(authedPage.locator('.top-menu-clear-all')).not.toBeVisible();
    expect(authedPage.url()).not.toContain('start_date=');
  });
});

test.describe('sort', () => {
  const seedThree = async (
    context: { request: APIRequestContext },
    apiOrigin: string,
  ) => {
    const groceries = await getCategoryId(
      context.request,
      apiOrigin,
      'Groceries',
    );
    await seedTransaction(context.request, apiOrigin, {
      date: '2023-02-01',
      merchant: 'Alpha Shop',
      amount: '10.00',
      categoryId: groceries,
    });
    await seedTransaction(context.request, apiOrigin, {
      date: '2023-02-15',
      merchant: 'Beta Shop',
      amount: '50.00',
      categoryId: groceries,
    });
    await seedTransaction(context.request, apiOrigin, {
      date: '2023-02-10',
      merchant: 'Gamma Shop',
      amount: '25.00',
      categoryId: groceries,
    });
  };

  const orders: {
    label: string;
    expectedMerchants: string[];
    isDefault?: boolean;
  }[] = [
    {
      label: 'Date (new to old)',
      expectedMerchants: ['Beta Shop', 'Gamma Shop', 'Alpha Shop'],
      isDefault: true,
    },
    {
      label: 'Date (old to new)',
      expectedMerchants: ['Alpha Shop', 'Gamma Shop', 'Beta Shop'],
    },
    {
      label: 'Amount (high to low)',
      expectedMerchants: ['Beta Shop', 'Gamma Shop', 'Alpha Shop'],
    },
    {
      label: 'Amount (low to high)',
      expectedMerchants: ['Alpha Shop', 'Gamma Shop', 'Beta Shop'],
    },
  ];

  for (const { label, expectedMerchants, isDefault } of orders) {
    test(`sorts by "${label}"`, async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedThree(context, workerInfra.apiOrigin);

      await authedPage.goto('/transactions');
      await authedPage
        .getByRole('button', { name: 'Sort', exact: true })
        .click();
      await authedPage
        .getByRole('button', { name: label, exact: true })
        .click();

      await expect(authedPage.locator('.transactions-row-merchant')).toHaveText(
        expectedMerchants,
      );

      const badge = authedPage.locator('.sort-button-badge');
      if (isDefault) await expect(badge).not.toBeVisible();
      else await expect(badge).toBeVisible();
    });
  }
});

test.describe('download', () => {
  test('closes the popover on Escape and returns focus to the trigger', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const trigger = authedPage.getByRole('button', {
      name: 'Download',
      exact: true,
    });
    const popover = authedPage.locator('.download-popover');

    await trigger.click();
    await expect(popover).toBeVisible();
    await authedPage.keyboard.press('Escape');
    await expect(popover).not.toBeVisible();
    await expect(trigger).toBeFocused();
  });

  test('closes the popover on an outside click and returns focus to the trigger', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    const trigger = authedPage.getByRole('button', {
      name: 'Download',
      exact: true,
    });
    const popover = authedPage.locator('.download-popover');

    await trigger.click();
    await expect(popover).toBeVisible();
    await authedPage.locator('.top-menu-title').click();
    await expect(popover).not.toBeVisible();
    await expect(trigger).toBeFocused();
  });

  test('downloads a CSV', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Download', exact: true })
      .click();
    const [download] = await Promise.all([
      authedPage.waitForEvent('download'),
      authedPage
        .getByRole('button', { name: 'Download as CSV', exact: true })
        .click(),
    ]);
    expect(download.suggestedFilename()).toMatch(/\.csv$/);
  });

  test('downloads an Excel file', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Download', exact: true })
      .click();
    const [download] = await Promise.all([
      authedPage.waitForEvent('download'),
      authedPage
        .getByRole('button', { name: 'Download as Excel', exact: true })
        .click(),
    ]);
    expect(download.suggestedFilename()).toMatch(/\.xlsx$/);
  });
});

test.describe('clear all', () => {
  test('is absent with no active filters', async ({ authedPage }) => {
    await authedPage.goto('/transactions');
    await expect(authedPage.locator('.top-menu-clear-all')).not.toBeVisible();
  });

  test('resets an active search, sort, and date range together', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');

    await authedPage
      .getByRole('button', { name: 'Search', exact: true })
      .click();
    await authedPage
      .getByPlaceholder('Enter a search term...')
      .fill('anything');
    await authedPage
      .locator('.search-popover')
      .getByRole('button', { name: 'Apply', exact: true })
      .click();

    await authedPage.getByRole('button', { name: 'Sort', exact: true }).click();
    await authedPage
      .getByRole('button', { name: 'Amount (high to low)', exact: true })
      .click();

    await authedPage.getByRole('button', { name: 'Date', exact: true }).click();
    await authedPage
      .getByRole('button', { name: 'Last 7 days', exact: true })
      .click();

    await authedPage.locator('.top-menu-clear-all').click();

    await expect(
      authedPage.getByRole('button', { name: 'Search', exact: true }),
    ).toBeVisible();
    await expect(authedPage.locator('.sort-button-badge')).not.toBeVisible();
    await expect(
      authedPage.getByRole('button', { name: 'Date', exact: true }),
    ).toBeVisible();
    await expect(authedPage.locator('.top-menu-clear-all')).not.toBeVisible();
    expect(authedPage.url()).not.toContain('search=');
    expect(authedPage.url()).not.toContain('order=');
    expect(authedPage.url()).not.toContain('start_date=');
  });
});

test.describe('edit 1 field directly', () => {
  const seedCornerStore = async (
    request: APIRequestContext,
    apiOrigin: string,
    categoryName: 'Groceries' | 'Coffee Shops' = 'Groceries',
  ): Promise<void> => {
    const categoryId = await getCategoryId(request, apiOrigin, categoryName);
    await seedTransaction(request, apiOrigin, {
      date: '2023-04-01',
      merchant: 'Corner Store',
      amount: '20.00',
      categoryId,
    });
  };

  test('rewrites the merchant on Enter', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row
      .getByRole('button', { name: 'Corner Store', exact: true })
      .click();
    const input = authedPage.locator('.transactions-row-input');
    await input.fill('Uptown Store');
    await input.press('Enter');

    const updatedRow = authedPage.locator('li.transactions-row', {
      hasText: 'Uptown Store',
    });
    await expect(
      updatedRow.getByRole('button', { name: 'Uptown Store', exact: true }),
    ).toBeVisible();
  });

  test('rewrites the account on blur', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.getByRole('button', { name: 'Seed', exact: true }).click();
    const input = authedPage.locator('.transactions-row-input');
    await input.fill('Chequing');
    // Clicking elsewhere blurs the input without pressing Enter.
    await authedPage.locator('.top-menu-title').click();

    await expect(
      row.getByRole('button', { name: 'Chequing', exact: true }),
    ).toBeVisible();
  });

  test('rewrites the amount', async ({ authedPage, context, workerInfra }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.locator('.transactions-row-amount').click();
    const input = authedPage.locator('.transactions-row-input--amount');
    await input.fill('35.50');
    await input.press('Enter');

    await expect(row.locator('.transactions-row-amount')).toContainText(
      money(35.5),
    );
  });

  test('strips a minus sign as it is typed, so amounts can never go negative', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.locator('.transactions-row-amount').click();
    const input = authedPage.locator('.transactions-row-input--amount');
    await input.fill('-35.50');

    await expect(input).toHaveValue('35.50');
  });

  test('cancels an edit on Escape without saving', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row
      .getByRole('button', { name: 'Corner Store', exact: true })
      .click();
    const input = authedPage.locator('.transactions-row-input');
    await input.fill('Should Not Save');
    await authedPage.keyboard.press('Escape');

    await expect(
      row.getByRole('button', { name: 'Corner Store', exact: true }),
    ).toBeVisible();
    await expect(authedPage.getByText('Should Not Save')).not.toBeVisible();
  });

  test('cancels an account edit on Escape without saving', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.getByRole('button', { name: 'Seed', exact: true }).click();
    const input = authedPage.locator('.transactions-row-input');
    await input.fill('Should Not Save');
    await authedPage.keyboard.press('Escape');

    await expect(
      row.getByRole('button', { name: 'Seed', exact: true }),
    ).toBeVisible();
    await expect(authedPage.getByText('Should Not Save')).not.toBeVisible();
  });

  test('cancels an amount edit on Escape without saving', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.locator('.transactions-row-amount').click();
    const input = authedPage.locator('.transactions-row-input--amount');
    await input.fill('999.99');
    await authedPage.keyboard.press('Escape');

    await expect(row.locator('.transactions-row-amount')).toContainText(
      money(20),
    );
    await expect(authedPage.getByText('999.99')).not.toBeVisible();
  });

  test('shows an error and keeps editing when the amount cannot be parsed', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.locator('.transactions-row-amount').click();
    const input = authedPage.locator('.transactions-row-input--amount');
    await input.fill('1,234');
    await input.press('Enter');

    await expect(
      authedPage.getByText(
        'Enter a plain amount, e.g. 12.50, without thousands separators.',
      ),
    ).toBeVisible();
    await expect(input).toBeVisible();
    await expect(input).toHaveValue('1,234');
  });

  test('shows a specific error and keeps editing when the amount is too long to store', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await row.locator('.transactions-row-amount').click();
    const input = authedPage.locator('.transactions-row-input--amount');
    await input.fill('12345678901.50');
    await input.press('Enter');

    await expect(
      authedPage.getByText(
        'Enter an amount with at most 10 digits before the decimal point.',
      ),
    ).toBeVisible();
    await expect(input).toBeVisible();
    await expect(input).toHaveValue('12345678901.50');
  });

  test.describe('when the update request fails', () => {
    const routeAllPatchesToFail = (page: Page) =>
      page.route('**/transactions/*', async (route) => {
        if (route.request().method() === 'PATCH') {
          await route.fulfill({
            status: 500,
            contentType: 'application/json',
            body: '{}',
          });
        } else {
          await route.continue();
        }
      });

    test('shows a toast and keeps the old merchant', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(context.request, workerInfra.apiOrigin);
      await authedPage.goto('/transactions');
      await routeAllPatchesToFail(authedPage);

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row
        .getByRole('button', { name: 'Corner Store', exact: true })
        .click();
      const input = authedPage.locator('.transactions-row-input');
      await input.fill('Uptown Store');
      await input.press('Enter');

      await expect(
        authedPage.getByText(
          'Failed to update the transaction. Please try again.',
        ),
      ).toBeVisible();
      await expect(
        row.getByRole('button', { name: 'Corner Store', exact: true }),
      ).toBeVisible();
    });

    test('shows a toast and keeps the old account', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(context.request, workerInfra.apiOrigin);
      await authedPage.goto('/transactions');
      await routeAllPatchesToFail(authedPage);

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.getByRole('button', { name: 'Seed', exact: true }).click();
      const input = authedPage.locator('.transactions-row-input');
      await input.fill('Chequing');
      await input.press('Enter');

      await expect(
        authedPage.getByText(
          'Failed to update the transaction. Please try again.',
        ),
      ).toBeVisible();
      await expect(
        row.getByRole('button', { name: 'Seed', exact: true }),
      ).toBeVisible();
    });

    test('shows a toast and keeps the old amount', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(context.request, workerInfra.apiOrigin);
      await authedPage.goto('/transactions');
      await routeAllPatchesToFail(authedPage);

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.locator('.transactions-row-amount').click();
      const input = authedPage.locator('.transactions-row-input--amount');
      await input.fill('35.50');
      await input.press('Enter');

      await expect(
        authedPage.getByText(
          'Failed to update the transaction. Please try again.',
        ),
      ).toBeVisible();
      await expect(row.locator('.transactions-row-amount')).toContainText(
        money(20),
      );
    });

    test('shows a toast and keeps the old category', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(
        context.request,
        workerInfra.apiOrigin,
        'Groceries',
      );
      await authedPage.goto('/transactions');
      await routeAllPatchesToFail(authedPage);

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.getByRole('button', { name: 'Groceries', exact: true }).click();
      await authedPage
        .locator('.category-picker-popover')
        .getByRole('button', { name: 'Coffee Shops', exact: true })
        .click();

      await expect(
        authedPage.getByText(
          'Failed to update the transaction. Please try again.',
        ),
      ).toBeVisible();
      await expect(
        row.getByRole('button', { name: 'Groceries', exact: true }),
      ).toBeVisible();
    });
  });

  test.describe('category', () => {
    test('opens a searchable list grouped by category group', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(
        context.request,
        workerInfra.apiOrigin,
        'Groceries',
      );
      await authedPage.goto('/transactions');

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.getByRole('button', { name: 'Groceries', exact: true }).click();

      const popover = authedPage.locator('.category-picker-popover');
      await expect(popover).toBeVisible();
      await expect(popover.getByText('Food & Dining')).toBeVisible();
      await expect(
        popover.getByRole('button', { name: 'Groceries', exact: true }),
      ).toBeVisible();
      await expect(
        popover.getByRole('button', { name: 'Coffee Shops', exact: true }),
      ).toBeVisible();
    });

    test('filters the list as you search', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(
        context.request,
        workerInfra.apiOrigin,
        'Groceries',
      );
      await authedPage.goto('/transactions');

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.getByRole('button', { name: 'Groceries', exact: true }).click();

      const popover = authedPage.locator('.category-picker-popover');
      await popover.getByPlaceholder('Search categories...').fill('coffee');

      await expect(
        popover.getByRole('button', { name: 'Coffee Shops', exact: true }),
      ).toBeVisible();
      await expect(
        popover.getByRole('button', { name: 'Groceries', exact: true }),
      ).not.toBeVisible();
    });

    test('selects a new category from the list', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(
        context.request,
        workerInfra.apiOrigin,
        'Groceries',
      );
      await authedPage.goto('/transactions');

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.getByRole('button', { name: 'Groceries', exact: true }).click();
      await authedPage
        .locator('.category-picker-popover')
        .getByRole('button', { name: 'Coffee Shops', exact: true })
        .click();

      await expect(
        row.getByRole('button', { name: 'Coffee Shops', exact: true }),
      ).toBeVisible();
    });

    test('shows a not-available-yet message for "Create new category"', async ({
      authedPage,
      context,
      workerInfra,
    }) => {
      await seedCornerStore(
        context.request,
        workerInfra.apiOrigin,
        'Groceries',
      );
      await authedPage.goto('/transactions');

      const row = authedPage.locator('li.transactions-row', {
        hasText: 'Corner Store',
      });
      await row.getByRole('button', { name: 'Groceries', exact: true }).click();
      await authedPage
        .locator('.category-picker-popover')
        .getByRole('button', { name: 'Create new category', exact: true })
        .click();

      await expect(
        authedPage.getByText('This feature is not available yet.'),
      ).toBeVisible();
      await expect(
        authedPage.locator('.category-picker-popover'),
      ).not.toBeVisible();
    });

    test.describe('keyboard navigation', () => {
      // Both default categories contain "income" (Business Income, Other Income), giving a
      // short, deterministic list — plus "Create new category" — to arrow through.
      const openFilteredToIncome = async (
        authedPage: Page,
        context: { request: APIRequestContext },
        workerInfra: { apiOrigin: string },
      ) => {
        await seedCornerStore(
          context.request,
          workerInfra.apiOrigin,
          'Groceries',
        );
        await authedPage.goto('/transactions');

        const row = authedPage.locator('li.transactions-row', {
          hasText: 'Corner Store',
        });
        await row
          .getByRole('button', { name: 'Groceries', exact: true })
          .click();
        await authedPage
          .locator('.category-picker-popover')
          .getByPlaceholder('Search categories...')
          .fill('income');
      };

      test('moves the highlight down with ArrowDown and selects with Enter', async ({
        authedPage,
        context,
        workerInfra,
      }) => {
        await openFilteredToIncome(authedPage, context, workerInfra);

        // Index 0 (Business Income) is highlighted as soon as the list narrows; one ArrowDown
        // moves to index 1 (Other Income).
        await authedPage.keyboard.press('ArrowDown');
        await authedPage.keyboard.press('Enter');

        await expect(
          authedPage.getByRole('button', { name: 'Other Income', exact: true }),
        ).toBeVisible();
      });

      test('does not move past the top with ArrowUp', async ({
        authedPage,
        context,
        workerInfra,
      }) => {
        await openFilteredToIncome(authedPage, context, workerInfra);

        await authedPage.keyboard.press('ArrowUp');
        await authedPage.keyboard.press('ArrowUp');
        await authedPage.keyboard.press('Enter');

        await expect(
          authedPage.getByRole('button', {
            name: 'Business Income',
            exact: true,
          }),
        ).toBeVisible();
      });

      test('moves past the last category to "Create new category" and can select it with Enter', async ({
        authedPage,
        context,
        workerInfra,
      }) => {
        await openFilteredToIncome(authedPage, context, workerInfra);

        // Only 2 categories match "income"; 3 ArrowDown presses overshoots onto (and clamps at)
        // "Create new category".
        await authedPage.keyboard.press('ArrowDown');
        await authedPage.keyboard.press('ArrowDown');
        await authedPage.keyboard.press('ArrowDown');
        await authedPage.keyboard.press('Enter');

        await expect(
          authedPage.getByText('This feature is not available yet.'),
        ).toBeVisible();
      });
    });
  });
});
