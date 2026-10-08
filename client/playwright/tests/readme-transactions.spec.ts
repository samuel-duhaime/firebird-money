/** Generates the screenshot shown in the README's Screenshots section (`npm run
 * screenshot:readme`). Tagged `@screenshot` so `test:ui` excludes it from the regular e2e run. */

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, expect } from '../fixtures';
import { getCategoryId, getTagId, seedTransaction } from '../lib/seed';

const here = path.dirname(fileURLToPath(import.meta.url));
const screenshotPath = path.resolve(
  here,
  '../../../docs/images/transactions-page.png',
);

/** A small, curated set of transactions built to look like a real household's activity —
 * unlike the arbitrary fixtures the e2e suite seeds, this is what ends up in the README. */
const DEMO_TRANSACTIONS: {
  date: string;
  merchant: string;
  amount: string;
  category: string;
  account: string;
  tags?: string[];
}[] = [
  {
    // Not "Whole Foods Market" — common merchants (seeded for every household, including this
    // worker's) now match substrings of raw statement text, and "Whole Foods" is one of them, so
    // that text would resolve to the common merchant and display as "Whole Foods" instead.
    date: '2026-06-14',
    merchant: 'Riverside Market',
    amount: '86.42',
    category: 'Groceries',
    account: 'Chequing',
    tags: ['Split'],
  },
  {
    date: '2026-06-14',
    merchant: 'Blue Bottle Coffee',
    amount: '5.75',
    category: 'Coffee Shops',
    account: 'Credit Card',
  },
  {
    date: '2026-06-13',
    merchant: 'Acme Corp Payroll',
    amount: '2400.00',
    category: 'Paychecks',
    account: 'Chequing',
  },
  {
    date: '2026-06-13',
    merchant: 'Netflix',
    amount: '16.99',
    category: 'Entertainment & Recreation',
    account: 'Credit Card',
    tags: ['Subscription'],
  },
  {
    date: '2026-06-13',
    merchant: 'Shell',
    amount: '54.30',
    category: 'Gas',
    account: 'Credit Card',
  },
  {
    date: '2026-06-10',
    merchant: 'Le Bistro',
    amount: '62.15',
    category: 'Restaurants & Bars',
    account: 'Credit Card',
    tags: ['Business'],
  },
  {
    date: '2026-06-10',
    merchant: 'Landlord Co',
    amount: '1450.00',
    category: 'Rent',
    account: 'Chequing',
  },
];

test('transactions page, for the README', { tag: '@screenshot' }, async ({
  authedPage,
  context,
  workerInfra,
}) => {
  for (const {
    date,
    merchant,
    amount,
    category,
    account,
    tags,
  } of DEMO_TRANSACTIONS) {
    const categoryId = await getCategoryId(
      context.request,
      workerInfra.apiOrigin,
      category,
    );
    const tagIds = tags
      ? await Promise.all(
          tags.map((tag) =>
            getTagId(context.request, workerInfra.apiOrigin, tag),
          ),
        )
      : undefined;
    await seedTransaction(context.request, workerInfra.apiOrigin, {
      date,
      merchant,
      amount,
      categoryId,
      account,
      tagIds,
    });
  }

  await authedPage.goto('/transactions');
  await expect(
    authedPage.locator('li.transactions-row', {
      hasText: 'Riverside Market',
    }),
  ).toBeVisible();

  // Dev-only overlays (React Query/Router devtools toggle, the empty toast container) render as
  // siblings of `.app-layout` under `#root` — real users never see them, so hide them for the
  // screenshot rather than shipping a screenshot of the dev server's chrome.
  await authedPage.addStyleTag({
    content: '#root > *:not(.app-layout) { display: none !important; }',
  });

  await authedPage.screenshot({ path: screenshotPath });
});
