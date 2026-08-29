import type { APIRequestContext } from '@playwright/test';
import { test, expect } from '../fixtures';
import {
  DATE_RANGE_PRESETS,
  resolvePreset,
} from '../../src/features/transactions/date-range';
import type { DateRangePreset } from '../../src/features/transactions/date-range';
import {
  formatAmount,
  formatDateHeading,
} from '../../src/features/transactions/format';

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

const getCategoryId = async (
  request: APIRequestContext,
  apiOrigin: string,
  nameEn: 'Groceries' | 'Salary' | 'Transfer',
): Promise<number> => {
  const response = await request.get(`${apiOrigin}/categories`);
  const categories: { id: number; name_en: string }[] = await response.json();
  const match = categories.find((category) => category.name_en === nameEn);
  if (!match) throw new Error(`category not seeded: ${nameEn}`);
  return match.id;
};

type SeedTransaction = {
  date: string;
  merchant: string;
  amount: string;
  categoryId: number;
};

const seedTransaction = async (
  request: APIRequestContext,
  apiOrigin: string,
  { date, merchant, amount, categoryId }: SeedTransaction,
): Promise<void> => {
  const response = await request.post(`${apiOrigin}/transactions`, {
    data: { date, merchant, amount, category_id: categoryId, account: 'Seed' },
  });
  if (!response.ok()) {
    throw new Error(
      `failed to seed transaction "${merchant}": ${response.status()} ${await response.text()}`,
    );
  }
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
    await expect(
      authedPage.getByText('Loading transactions…'),
    ).toBeVisible();
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
      'Salary',
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
  test('closes via Escape, the close button, and Cancel', async ({
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

    await addButton.click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).not.toBeVisible();

    await addButton.click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).not.toBeVisible();
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

    // The × close button is the dialog's first focusable element.
    await expect(closeButton).toBeFocused();

    // Shift+Tab from the first element wraps around to the last.
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
    await dialog.getByLabel('Category').selectOption({ label: 'Groceries' });
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
    await dialog.getByLabel('Category').selectOption({ label: 'Groceries' });
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
      await dialog.getByLabel('Category').selectOption({ label: 'Groceries' });
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
    await authedPage
      .getByPlaceholder('Enter a search term...')
      .fill('foods');
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
