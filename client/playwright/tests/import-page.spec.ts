import type { Page, Route } from '@playwright/test';
import { test, expect } from '../fixtures';
import { ensureAccount, getCategoryId } from '../lib/seed';

/**
 * The statement import page, end to end — with the Claude read always mocked.
 *
 * `POST /transactions/import/read` is intercepted in the browser (`mockRead`), so no test ever
 * reaches Claude. Belt and braces: the test server runs with `ANTHROPIC_API_KEY` blank (see
 * `lib/processes.ts`), so even an unmocked read can only answer `503` — which one test relies on.
 * `POST /transactions/import` (adding the accepted lines) runs against the real test server.
 */

const READ_URL = '**/transactions/import/read';

type Line = {
  date: string;
  original_statement: string;
  amount: string;
  merchant_id: number | null;
  new_merchant_name: string | null;
  category_id: number | null;
  category_confident: boolean;
  account_id: number | null;
  new_account: {
    institution_name: string;
    name: string;
    type: string;
    last_four: string | null;
  } | null;
  tag_ids: number[];
  duplicate_of_transaction_id: number | null;
};

const line = (overrides: Partial<Line>): Line => ({
  date: '2026-09-02',
  original_statement: 'EPICERIE CHEZ PAUL #12 MONTREAL',
  amount: '142.37',
  merchant_id: null,
  new_merchant_name: 'Épicerie Chez Paul',
  category_id: null,
  category_confident: true,
  account_id: null,
  new_account: null,
  tag_ids: [],
  duplicate_of_transaction_id: null,
  ...overrides,
});

/** Answers the read with these progress events, as the server's newline-delimited JSON. */
const mockRead = (page: Page, events: object[]) =>
  page.route(READ_URL, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/x-ndjson',
      body: events.map((event) => JSON.stringify(event)).join('\n') + '\n',
    }),
  );

/** A complete, successful read that returns `lines`. */
const mockReadDone = (page: Page, lines: Line[]) =>
  mockRead(page, [
    { type: 'reading' },
    { type: 'found', count: lines.length },
    { type: 'matching' },
    { type: 'checking_duplicates' },
    { type: 'done', transactions: lines },
  ]);

const uploadPdf = (page: Page, name = 'statement-september.pdf') =>
  page.locator('input[type="file"]').setInputFiles({
    name,
    mimeType: 'application/pdf',
    buffer: Buffer.from('%PDF-1.7 test statement'),
  });

/** The dev-only devtools toggles render fixed over the review footer's buttons. */
const hideDevtools = (page: Page) =>
  page.addStyleTag({
    content: `
      .tsqd-parent-container,
      button[aria-label="Open TanStack Router Devtools"] {
        display: none !important;
      }
    `,
  });

const reviewRow = (page: Page, merchant: string) =>
  page.locator('tr.statement-row', { hasText: merchant });

test.describe('import page: upload', () => {
  test('the Import button opens the page, with the drop zone and privacy card', async ({
    authedPage,
  }) => {
    await authedPage.goto('/transactions');
    await authedPage
      .getByRole('button', { name: 'Import', exact: true })
      .click();

    await expect(authedPage).toHaveURL(/\/transactions\/import$/);
    await expect(
      authedPage.getByRole('heading', { name: 'Import a statement' }),
    ).toBeVisible();
    await expect(
      authedPage.getByText('Drag your statement here'),
    ).toBeVisible();
    await expect(
      authedPage.getByRole('heading', { name: 'Your statement stays private' }),
    ).toBeVisible();
    await expect(authedPage.getByText('Never saved')).toBeVisible();
    await expect(authedPage.getByText('Not used for training')).toBeVisible();
    await expect(authedPage.getByText('You decide')).toBeVisible();
    await expect(authedPage.getByLabel('Account')).toHaveValue('');
  });

  test('refuses a file over 10 MB without sending it', async ({
    authedPage,
  }) => {
    let requested = false;
    await authedPage.route(READ_URL, (route) => {
      requested = true;
      return route.abort();
    });
    await authedPage.goto('/transactions/import');

    await authedPage.locator('input[type="file"]').setInputFiles({
      name: 'huge.pdf',
      mimeType: 'application/pdf',
      buffer: Buffer.alloc(10 * 1024 * 1024 + 1, 'a'),
    });

    await expect(authedPage.getByRole('alert')).toHaveText(
      'This file is too large (10 MB max).',
    );
    expect(requested).toBe(false);
  });

  test("shows the server's message when the file type isn't supported", async ({
    authedPage,
  }) => {
    await authedPage.route(READ_URL, (route) =>
      route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({
          error:
            'Only PDF, image (PNG, JPG, GIF, WebP), CSV, and Excel files can be imported',
        }),
      }),
    );
    await authedPage.goto('/transactions/import');

    await uploadPdf(authedPage, 'notes.docx');

    await expect(authedPage.getByRole('alert')).toContainText(
      'Only PDF, image (PNG, JPG, GIF, WebP), CSV, and Excel files can be imported',
    );
    await expect(
      authedPage.getByText('Drag your statement here'),
    ).toBeVisible();
  });

  test('says when statement import is off on the server (no API key)', async ({
    authedPage,
  }) => {
    // Deliberately not mocked: the test server has no API key, so it answers 503 up front.
    await authedPage.goto('/transactions/import');

    await uploadPdf(authedPage);

    await expect(authedPage.getByRole('alert')).toContainText(
      'Statement import isn’t set up on this server',
    );
  });

  test('sends the picked account along with the file', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const accountId = await ensureAccount(
      context.request,
      workerInfra.apiOrigin,
      'Chequing',
    );
    let sentAccount: string | null = null;
    await authedPage.route(READ_URL, async (route: Route) => {
      sentAccount =
        route
          .request()
          .postData()
          ?.match(/name="account_id"\r\n\r\n(\d+)/)?.[1] ?? null;
      await route.fulfill({
        status: 200,
        contentType: 'application/x-ndjson',
        body: JSON.stringify({ type: 'done', transactions: [] }) + '\n',
      });
    });
    await authedPage.goto('/transactions/import');

    await authedPage.getByLabel('Account').selectOption({ label: 'Chequing' });
    await uploadPdf(authedPage);

    await expect(
      authedPage.getByText('No transactions were found in this statement.'),
    ).toBeVisible();
    expect(sentAccount).toBe(String(accountId));
  });
});

test.describe('import page: reading', () => {
  test('shows the file and progress while reading, and Cancel goes back', async ({
    authedPage,
  }) => {
    // Never answered, so the page stays on the progress step.
    await authedPage.route(READ_URL, () => new Promise(() => {}));
    await authedPage.goto('/transactions/import');

    await uploadPdf(authedPage);

    await expect(authedPage.getByText('statement-september.pdf')).toBeVisible();
    await expect(
      authedPage.getByText('Read in memory, not saved'),
    ).toBeVisible();
    await expect(authedPage.getByText('Reading the statement')).toBeVisible();
    await expect(
      authedPage.getByText('Matching to your household'),
    ).toBeVisible();
    await expect(authedPage.getByText('Checking for duplicates')).toBeVisible();

    await authedPage.getByRole('button', { name: 'Cancel' }).click();

    await expect(
      authedPage.getByText('Drag your statement here'),
    ).toBeVisible();
    await expect(authedPage.getByRole('alert')).toHaveCount(0);
  });

  test('an error event mid-read goes back to the upload step with its message', async ({
    authedPage,
  }) => {
    await mockRead(authedPage, [
      { type: 'reading' },
      { type: 'found', count: 2 },
      { type: 'error', error: 'No transactions could be read from this file' },
    ]);
    await authedPage.goto('/transactions/import');

    await uploadPdf(authedPage);

    await expect(authedPage.getByRole('alert')).toContainText(
      'No transactions could be read from this file',
    );
  });

  test('a stream that ends without a result shows a generic error', async ({
    authedPage,
  }) => {
    await mockRead(authedPage, [{ type: 'reading' }, { type: 'matching' }]);
    await authedPage.goto('/transactions/import');

    await uploadPdf(authedPage);

    await expect(authedPage.getByRole('alert')).toContainText(
      "The statement couldn't be read. Please try again.",
    );
  });

  test('a statement with no transactions offers to try another file', async ({
    authedPage,
  }) => {
    await mockReadDone(authedPage, []);
    await authedPage.goto('/transactions/import');

    await uploadPdf(authedPage);
    await expect(
      authedPage.getByText('No transactions were found in this statement.'),
    ).toBeVisible();
    await authedPage.getByRole('button', { name: 'Try another file' }).click();

    await expect(
      authedPage.getByText('Drag your statement here'),
    ).toBeVisible();
  });
});

test.describe('import page: review', () => {
  test('lists the lines with their hints, filters, and totals', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const accountId = await ensureAccount(
      context.request,
      workerInfra.apiOrigin,
      'Chequing',
    );
    await mockReadDone(authedPage, [
      line({ category_id: groceries, account_id: accountId }),
      line({
        date: '2026-09-14',
        original_statement: 'CAFE OLIMPICO MTL',
        amount: '9.40',
        new_merchant_name: 'Café Olimpico',
        category_id: groceries,
        category_confident: false,
        account_id: accountId,
      }),
      line({
        date: '2026-09-12',
        original_statement: 'METRO PLUS ROSEMONT',
        amount: '87.10',
        new_merchant_name: 'Metro',
        category_id: groceries,
        account_id: accountId,
        duplicate_of_transaction_id: 42,
      }),
    ]);
    await authedPage.goto('/transactions/import');
    await uploadPdf(authedPage);

    await expect(
      authedPage.getByRole('heading', { name: 'Review 3 transactions' }),
    ).toBeVisible();
    await expect(
      authedPage.getByText('statement-september.pdf · file already discarded'),
    ).toBeVisible();

    // The raw statement text and the hints.
    await expect(reviewRow(authedPage, 'Chez Paul')).toContainText(
      'EPICERIE CHEZ PAUL #12 MONTREAL',
    );
    await expect(reviewRow(authedPage, 'Chez Paul')).toContainText(
      'New merchant',
    );
    await expect(reviewRow(authedPage, 'Café Olimpico')).toContainText(
      'Not sure, please check',
    );
    await expect(reviewRow(authedPage, 'Metro')).toContainText(
      'Already in your transactions. Left unticked.',
    );

    // A likely duplicate starts unticked; the rest start ticked.
    await expect(
      reviewRow(authedPage, 'Metro').getByRole('checkbox'),
    ).not.toBeChecked();
    await expect(
      reviewRow(authedPage, 'Chez Paul').getByRole('checkbox'),
    ).toBeChecked();
    await expect(authedPage.getByText('2 of 3 selected')).toBeVisible();
    await expect(
      authedPage.getByText('2 new merchants will be created'),
    ).toBeVisible();
    await expect(
      authedPage.getByRole('button', { name: 'Add 2 transactions' }),
    ).toBeEnabled();

    // Filters.
    await authedPage
      .getByRole('button', { name: /Possible duplicates/ })
      .click();
    await expect(authedPage.locator('tr.statement-row')).toHaveCount(1);
    await expect(reviewRow(authedPage, 'Metro')).toBeVisible();
    await authedPage.getByRole('button', { name: /Needs a look/ }).click();
    await expect(authedPage.locator('tr.statement-row')).toHaveCount(3);
    await authedPage.getByRole('button', { name: /^All/ }).click();
    await expect(authedPage.locator('tr.statement-row')).toHaveCount(3);

    // Select all, then none.
    const selectAll = authedPage.getByRole('checkbox', {
      name: 'Select all shown',
    });
    await selectAll.check();
    await expect(authedPage.getByText('3 of 3 selected')).toBeVisible();
    await selectAll.uncheck();
    await expect(authedPage.getByText('0 of 3 selected')).toBeVisible();
    await expect(
      authedPage.getByRole('button', { name: 'Add 0 transactions' }),
    ).toBeDisabled();
  });

  test("can't add a line missing its category or account until it's filled in", async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    await ensureAccount(context.request, workerInfra.apiOrigin, 'Chequing');
    await mockReadDone(authedPage, [line({})]);
    await authedPage.goto('/transactions/import');
    await uploadPdf(authedPage);
    await hideDevtools(authedPage);

    await expect(
      authedPage.getByText('1 selected line needs a category or account'),
    ).toBeVisible();
    const add = authedPage.getByRole('button', { name: 'Add 1 transaction' });
    await expect(add).toBeDisabled();

    const row = reviewRow(authedPage, 'Chez Paul');
    await row
      .getByRole('button', { name: 'Category: Choose a category' })
      .click();
    await authedPage
      .locator('.category-picker-popover')
      .getByRole('button', { name: 'Groceries', exact: true })
      .click();
    await expect(add).toBeDisabled();

    await row.getByLabel('Account').selectOption({ label: 'Chequing' });

    await expect(
      authedPage.getByText('1 selected line needs a category or account'),
    ).toHaveCount(0);
    await expect(add).toBeEnabled();
  });

  test('Discard goes back to the upload step', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    await mockReadDone(authedPage, [line({ category_id: groceries })]);
    await authedPage.goto('/transactions/import');
    await uploadPdf(authedPage);
    await hideDevtools(authedPage);

    await authedPage.getByRole('button', { name: 'Discard' }).click();

    await expect(
      authedPage.getByText('Drag your statement here'),
    ).toBeVisible();
  });

  test('adds the selected lines and lands on Transactions with a summary', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const accountId = await ensureAccount(
      context.request,
      workerInfra.apiOrigin,
      'Chequing',
    );
    await mockReadDone(authedPage, [
      line({ category_id: groceries, account_id: accountId }),
      line({
        date: '2026-09-14',
        original_statement: 'CAFE OLIMPICO MTL',
        amount: '9.40',
        new_merchant_name: 'Café Olimpico',
        category_id: groceries,
        account_id: accountId,
      }),
      line({
        date: '2026-09-12',
        original_statement: 'METRO PLUS ROSEMONT',
        amount: '87.10',
        new_merchant_name: 'Metro',
        category_id: groceries,
        account_id: accountId,
        duplicate_of_transaction_id: 42,
      }),
    ]);
    await authedPage.goto('/transactions/import');
    await uploadPdf(authedPage);
    await hideDevtools(authedPage);

    await authedPage
      .getByRole('button', { name: 'Add 2 transactions' })
      .click();

    await expect(authedPage).toHaveURL(/\/transactions$/);
    await expect(
      authedPage.getByText('2 transactions added · 2 new merchants'),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Café Olimpico' }),
    ).toBeVisible();
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Chez Paul' }),
    ).toBeVisible();
    // The unticked duplicate wasn't added.
    await expect(
      authedPage.locator('li.transactions-row', { hasText: 'Metro' }),
    ).toHaveCount(0);

    const transactions: { source: string }[] = await (
      await context.request.get(`${workerInfra.apiOrigin}/transactions`)
    ).json();
    expect(transactions.map((t) => t.source)).toEqual(['import', 'import']);
  });

  test('suggests and creates an account the statement names', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const mastercard = {
      institution_name: 'RBC',
      name: 'Mastercard',
      type: 'credit_card',
      last_four: '9999',
    };
    await mockReadDone(authedPage, [
      line({ category_id: groceries, new_account: mastercard }),
    ]);
    await authedPage.goto('/transactions/import');
    await uploadPdf(authedPage);
    await hideDevtools(authedPage);

    const row = reviewRow(authedPage, 'Chez Paul');
    await expect(row.getByLabel('Account')).toHaveValue('new');
    await expect(row.getByLabel('Account')).toContainText(
      'New: RBC Mastercard (…9999)',
    );
    await expect(row).toContainText('New account');
    await expect(
      authedPage.getByText('1 new account will be created'),
    ).toBeVisible();

    await authedPage.getByRole('button', { name: 'Add 1 transaction' }).click();

    await expect(
      authedPage.getByText(
        '1 transaction added · 1 new merchant · 1 new account',
      ),
    ).toBeVisible();
    const accounts: {
      name: string;
      institution_name: string;
      last_four: string;
    }[] = await (
      await context.request.get(`${workerInfra.apiOrigin}/accounts`)
    ).json();
    expect(accounts).toEqual([
      expect.objectContaining({
        name: 'Mastercard',
        // "RBC" is an alias of the common institution, reused rather than duplicated.
        institution_name: 'RBC Royal Bank',
        last_four: '9999',
      }),
    ]);
  });

  test('a failed add shows an error and keeps the review', async ({
    authedPage,
    context,
    workerInfra,
  }) => {
    const groceries = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      'Groceries',
    );
    const accountId = await ensureAccount(
      context.request,
      workerInfra.apiOrigin,
      'Chequing',
    );
    await mockReadDone(authedPage, [
      line({ category_id: groceries, account_id: accountId }),
    ]);
    // Scoped to the API: a bare `**/transactions/import` would also catch the page's own URL.
    await authedPage.route(
      `${workerInfra.apiOrigin}/transactions/import`,
      (route) =>
        route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: '{}',
        }),
    );
    await authedPage.goto('/transactions/import');
    await uploadPdf(authedPage);
    await hideDevtools(authedPage);

    await authedPage.getByRole('button', { name: 'Add 1 transaction' }).click();

    await expect(
      authedPage.getByText("Couldn't add the transactions. Please try again."),
    ).toBeVisible();
    await expect(authedPage).toHaveURL(/\/transactions\/import$/);
    await expect(reviewRow(authedPage, 'Chez Paul')).toBeVisible();
  });
});
