import { test, expect } from '../fixtures';

const DEFAULT_TAG_NAMES = [
  'Tax',
  'Reimburse',
  'Split',
  'Business',
  'Subscription',
  'Member #1',
];

test.describe('Household > Tags settings', () => {
  test('left nav links to Household, and lands on the Tags page', async ({
    authedPage,
  }) => {
    await authedPage.goto('/dashboard');
    await authedPage
      .getByRole('link', { name: 'Household', exact: true })
      .click();

    await expect(authedPage).toHaveURL('/settings/tags');
    await expect(authedPage.locator('.top-menu-title')).toHaveText('Settings');
    await expect(
      authedPage.getByRole('link', { name: 'Tags', exact: true }),
    ).toHaveClass(/settings-nav-link--active/);
  });

  test('lists the seeded starter tags, each with a transaction count', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/tags');

    for (const name of DEFAULT_TAG_NAMES) {
      const row = authedPage.locator('li.tags-list-row', { hasText: name });
      await expect(row).toBeVisible();
      await expect(row).toContainText('0 transactions');
    }
  });

  test('creates a tag through the New tag modal', async ({ authedPage }) => {
    await authedPage.goto('/settings/tags');
    await authedPage
      .getByRole('button', { name: 'New tag', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New tag' });
    await dialog.getByLabel('Color & Name').fill('Vacation 2026');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    await expect(
      authedPage.locator('li.tags-list-row', { hasText: 'Vacation 2026' }),
    ).toBeVisible();
  });

  test('rejects a duplicate tag name with an inline error, not a toast', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/tags');
    await authedPage
      .getByRole('button', { name: 'New tag', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New tag' });
    await dialog.getByLabel('Color & Name').fill('Tax');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog.getByRole('alert')).toHaveText(
      'A tag with this name already exists.',
    );
    await expect(dialog).toBeVisible();
  });

  test('requires a name before saving', async ({ authedPage }) => {
    await authedPage.goto('/settings/tags');
    await authedPage
      .getByRole('button', { name: 'New tag', exact: true })
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New tag' });
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog.getByRole('alert')).toHaveText('A name is required.');
    await expect(dialog).toBeVisible();
  });

  test('edits a tag through the Edit tag modal, prefilled with its current name', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/tags');
    const row = authedPage.locator('li.tags-list-row', { hasText: 'Tax' });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Edit tag' });
    await expect(dialog.getByLabel('Color & Name')).toHaveValue('Tax');
    await dialog.getByLabel('Color & Name').fill('Taxes');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    // Exact match, not `hasText` — "Tax" is a substring of "Taxes", so a substring filter can't
    // tell a renamed row from the old one.
    await expect(authedPage.getByText('Taxes', { exact: true })).toBeVisible();
    await expect(
      authedPage.getByText('Tax', { exact: true }),
    ).not.toBeVisible();
  });

  test('cancelling the modal discards changes', async ({ authedPage }) => {
    await authedPage.goto('/settings/tags');
    const row = authedPage.locator('li.tags-list-row', { hasText: 'Tax' });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'Edit tag' });
    await dialog.getByLabel('Color & Name').fill('Should not save');
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();

    await expect(dialog).not.toBeVisible();
    await expect(row).toContainText('Tax');
  });

  test('deleting shows an inline confirm panel, cancel keeps the tag', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/tags');
    const row = authedPage.locator('li.tags-list-row', {
      hasText: 'Member #1',
    });
    await row.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(row.getByText('Delete the "Member #1" tag?')).toBeVisible();
    await row.getByRole('button', { name: 'Cancel', exact: true }).click();

    await expect(
      row.getByText('Delete the "Member #1" tag?'),
    ).not.toBeVisible();
    await expect(row).toBeVisible();
  });

  test('confirming delete removes the tag', async ({ authedPage }) => {
    await authedPage.goto('/settings/tags');
    const row = authedPage.locator('li.tags-list-row', {
      hasText: 'Member #1',
    });
    await row.getByRole('button', { name: 'Delete', exact: true }).click();
    await row.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(row).not.toBeVisible();
  });

  test('drag-and-drop reorders tags, and the new order survives a reload', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/tags');
    const nameLocator = authedPage.locator('.tags-list-name');
    await expect(nameLocator).toHaveCount(DEFAULT_TAG_NAMES.length);
    await expect(nameLocator.first()).toHaveText('Tax');

    const rows = authedPage.locator('li.tags-list-row');
    await rows.nth(0).locator('.tags-list-drag-handle').dragTo(rows.nth(3));

    const afterDrag = await nameLocator.allTextContents();
    expect(afterDrag[0]).not.toBe('Tax');
    expect(afterDrag).toContain('Tax');
    expect(afterDrag).toHaveLength(DEFAULT_TAG_NAMES.length);

    await authedPage.reload();
    await expect(nameLocator).toHaveCount(DEFAULT_TAG_NAMES.length);
    await expect(nameLocator).toHaveText(afterDrag);
  });

  test('move up/down buttons reorder tags for anyone who cannot drag-and-drop, and survive a reload', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/tags');
    const nameLocator = authedPage.locator('.tags-list-name');
    await expect(nameLocator.first()).toHaveText('Tax');

    const firstRow = authedPage.locator('li.tags-list-row').first();
    // The first row can't move up — only down.
    await expect(
      firstRow.getByRole('button', { name: 'Move Tax up' }),
    ).toBeDisabled();
    await firstRow.getByRole('button', { name: 'Move Tax down' }).click();

    await expect(nameLocator.first()).not.toHaveText('Tax');
    await expect(nameLocator.nth(1)).toHaveText('Tax');

    // Move it back up with the row's own "move up" button, now that it isn't first anymore.
    await authedPage
      .locator('li.tags-list-row', { hasText: 'Tax' })
      .getByRole('button', { name: 'Move Tax up' })
      .click();
    await expect(nameLocator.first()).toHaveText('Tax');

    const afterMoves = await nameLocator.allTextContents();
    await authedPage.reload();
    await expect(nameLocator).toHaveText(afterMoves);
  });

  test('the last row cannot move further down', async ({ authedPage }) => {
    await authedPage.goto('/settings/tags');
    const lastRow = authedPage.locator('li.tags-list-row').last();
    await expect(
      lastRow.getByRole('button', { name: /^Move .+ down$/ }),
    ).toBeDisabled();
  });
});
