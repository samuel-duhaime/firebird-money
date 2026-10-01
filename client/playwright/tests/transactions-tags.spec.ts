import type { APIRequestContext, Page } from '@playwright/test';
import { test, expect } from '../fixtures';
import { getCategoryId, getTagId, seedTransaction } from '../lib/seed';

/** The tags field is the same multi-select `TagPicker` popover everywhere it appears (add form,
 * edit panel, edit-multiple panel, and inline in the transactions-list row) — portaled to
 * `document.body`, so it's queried against `page`, not whatever dialog/row contains the trigger.
 * Unlike `CategoryPicker`, picking a tag never closes the popover (multi-select), so the trigger
 * is clicked again afterward to close it. */
const pickTag = async (
  page: Page,
  trigger: ReturnType<Page['getByRole']>,
  tagName: string,
): Promise<void> => {
  await trigger.click();
  await page
    .locator('.tag-picker-popover')
    .getByRole('button', { name: tagName, exact: true })
    .click();
  await trigger.click();
};

/** The dev-only React Query/Router devtools toggles render fixed over panel footers and, given
 * enough steps in a test, have time to mount there — hide them before clicking a footer button,
 * mirroring `transactions-page.spec.ts`'s own `hideDevtools`. */
const hideDevtools = (page: Page) =>
  page.addStyleTag({
    content: `
      .tsqd-parent-container,
      button[aria-label="Open TanStack Router Devtools"] {
        display: none !important;
      }
    `,
  });

test.describe('add transaction with tags', () => {
  test('adding a transaction with tags shows them as chips in the row', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    await dialog.getByLabel('Amount').fill('12.50');
    await dialog.getByLabel('Merchant').fill('Corner Store');
    await dialog.getByLabel('Date').fill('2023-03-01');
    await dialog
      .getByRole('button', { name: 'Select category', exact: true })
      .click();
    await authedPage
      .locator('.category-picker-popover')
      .getByRole('button', { name: 'Groceries', exact: true })
      .click();

    const tagsTrigger = dialog.getByRole('button', {
      name: 'Tags',
      exact: true,
    });
    await pickTag(authedPage, tagsTrigger, 'Tax');
    await pickTag(authedPage, tagsTrigger, 'Business');

    await dialog
      .getByRole('button', { name: 'Add transaction', exact: true })
      .click();

    await expect(dialog).not.toBeVisible();
    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await expect(row).toContainText('Tax');
    await expect(row).toContainText('Business');
  });

  test('tags are optional — a transaction can be added with none', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage.getByRole('button', { name: 'Add', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Add transaction' });
    await dialog.getByLabel('Amount').fill('12.50');
    await dialog.getByLabel('Merchant').fill('No Tags Store');
    await dialog.getByLabel('Date').fill('2023-03-01');
    await dialog
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
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'No Tags Store' }),
    ).toBeVisible();
  });
});

test.describe('transactions list tags column', () => {
  test('shows each transaction’s attached tags, and none for an untagged one', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const taxId = await getTagId(context.request, workerInfra.apiOrigin, 'Tax');
    // Names deliberately share no substring (unlike e.g. "Tagged Store"/"Untagged Store") so a
    // `hasText` filter for one can't also match the other.
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Alpha Market',
      amount: '20.00',
      categoryId: groceries,
      tagIds: [taxId],
    });
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-02',
      merchant: 'Beta Bazaar',
      amount: '15.00',
      categoryId: groceries,
    });

    await authedPage.goto('/transactions');

    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Alpha Market' }),
    ).toContainText('Tax');
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Beta Bazaar' }),
    ).not.toContainText('Tax');
  });
});

test.describe('edit transaction tags', () => {
  const seedCornerStore = async (
    request: APIRequestContext,
    apiOrigin: string,
  ): Promise<void> => {
    const categoryId = await getCategoryId(request, apiOrigin, 'Groceries');
    const taxId = await getTagId(request, apiOrigin, 'Tax');
    await seedTransaction(request, apiOrigin, {
      date: '2023-04-01',
      merchant: 'Corner Store',
      amount: '20.00',
      categoryId,
      tagIds: [taxId],
    });
  };

  const openEditPanel = async (authedPage: Page) => {
    await authedPage.goto('/transactions');
    await authedPage
      .locator('li.transactions-row', { hasText: 'Corner Store' })
      .getByRole('button', { name: 'Edit transaction', exact: true })
      .click();
    return authedPage.getByRole('dialog', { name: 'Edit transaction' });
  };

  test('prefills with the transaction’s existing tags', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await expect(
      dialog.getByRole('button', { name: 'Tags', exact: true }),
    ).toContainText('Tax');
  });

  test('adding a tag autosaves immediately, replacing nothing already attached', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);

    await pickTag(
      authedPage,
      dialog.getByRole('button', { name: 'Tags', exact: true }),
      'Business',
    );

    const tagsTrigger = dialog.getByRole('button', {
      name: 'Tags',
      exact: true,
    });
    await expect(tagsTrigger).toContainText('Tax');
    await expect(tagsTrigger).toContainText('Business');

    const row = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    await expect(row).toContainText('Tax');
    await expect(row).toContainText('Business');
  });

  test('removing a tag (clicking it again) autosaves immediately', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedCornerStore(context.request, workerInfra.apiOrigin);
    const dialog = await openEditPanel(authedPage);
    const tagsTrigger = dialog.getByRole('button', {
      name: 'Tags',
      exact: true,
    });

    await pickTag(authedPage, tagsTrigger, 'Tax');

    await expect(tagsTrigger).not.toContainText('Tax');
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Corner Store' }),
    ).not.toContainText('Tax');
  });
});

test.describe('edit multiple transactions — tags add on top, never replace', () => {
  const seedTwo = async (
    request: APIRequestContext,
    apiOrigin: string,
  ): Promise<void> => {
    const groceriesId = await getCategoryId(request, apiOrigin, 'Groceries');
    const taxId = await getTagId(request, apiOrigin, 'Tax');
    await seedTransaction(request, apiOrigin, {
      date: '2023-04-01',
      merchant: 'Corner Store',
      amount: '20.00',
      categoryId: groceriesId,
      tagIds: [taxId],
    });
    await seedTransaction(request, apiOrigin, {
      date: '2023-04-02',
      merchant: 'Downtown Store',
      amount: '15.00',
      categoryId: groceriesId,
    });
  };

  test('shows an "adds to existing tags" hint next to the field label', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedTwo(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Edit multiple', exact: true })
      .click();
    await authedPage
      .locator('li.transactions-row', { hasText: 'Corner Store' })
      .getByRole('checkbox', { name: 'Select Corner Store', exact: true })
      .click();
    await authedPage
      .getByRole('button', { name: 'Edit 1', exact: true })
      .click();

    await expect(
      authedPage.getByText('adds to existing tags, never removes them'),
    ).toBeVisible();
  });

  test('adding a tag to several selected transactions keeps each one’s existing tags', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await seedTwo(context.request, workerInfra.apiOrigin);
    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Edit multiple', exact: true })
      .click();
    await authedPage
      .locator('li.transactions-row', { hasText: 'Corner Store' })
      .getByRole('checkbox', { name: 'Select Corner Store', exact: true })
      .click();
    await authedPage
      .locator('li.transactions-row', { hasText: 'Downtown Store' })
      .getByRole('checkbox', { name: 'Select Downtown Store', exact: true })
      .click();
    await authedPage
      .getByRole('button', { name: 'Edit 2', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', {
      name: 'Edit 2 transactions',
    });
    await pickTag(
      authedPage,
      dialog.getByRole('button', { name: 'Tags', exact: true }),
      'Business',
    );
    await hideDevtools(authedPage);
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    const cornerStoreRow = authedPage.locator('li.transactions-row', {
      hasText: 'Corner Store',
    });
    const downtownStoreRow = authedPage.locator('li.transactions-row', {
      hasText: 'Downtown Store',
    });
    // Corner Store already had Tax — it must still have it, plus the newly added Business.
    await expect(cornerStoreRow).toContainText('Tax');
    await expect(cornerStoreRow).toContainText('Business');
    // Downtown Store had no tags — it only gets Business.
    await expect(downtownStoreRow).toContainText('Business');
  });
});

test.describe('search matches tag names', () => {
  test('a search term matching only a tag name finds that transaction', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const taxId = await getTagId(context.request, workerInfra.apiOrigin, 'Tax');
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date: '2023-04-01',
      merchant: 'Whole Foods Market',
      amount: '30.00',
      categoryId: groceries,
      tagIds: [taxId],
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
    await authedPage.getByPlaceholder('Enter a search term...').fill('tax');
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
  });
});
