import { test, expect } from '../fixtures';

test.describe('Household > Categories settings', () => {
  test('left nav links to Household, and lands on the Categories page', async ({
    authedPage,
  }) => {
    await authedPage.goto('/dashboard');
    await authedPage
      .getByRole('link', { name: 'Household', exact: true })
      .click();
    await authedPage
      .getByRole('link', { name: 'Categories', exact: true })
      .click();

    await expect(authedPage).toHaveURL('/settings/categories');
    await expect(authedPage.locator('.top-menu-title')).toHaveText(
      'Settings',
    );
    await expect(
      authedPage.getByRole('link', { name: 'Categories', exact: true }),
    ).toHaveClass(/settings-nav-link--active/);
  });

  test('lists the seeded starter groups, grouped by type', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');

    // The section headings are CSS `text-transform: uppercase`, not actually-uppercase text
    // content — matched here by their real text, scoped to the heading so this doesn't also match
    // the "Income" group card title underneath it.
    await expect(
      authedPage.locator('.category-type-section-header', {
        hasText: 'Income',
      }),
    ).toBeVisible();
    await expect(
      authedPage.locator('.category-type-section-header', {
        hasText: 'Expenses',
      }),
    ).toBeVisible();
    await expect(
      authedPage.locator('.category-type-section-header', {
        hasText: 'Transfers',
      }),
    ).toBeVisible();
    await expect(
      authedPage.locator('.category-group-card', { hasText: 'Income' }),
    ).toBeVisible();
    await expect(
      authedPage.locator('.category-group-card', {
        hasText: 'Food & Dining',
      }),
    ).toBeVisible();
  });

  test('creating a category with only an English name auto-fills the French name', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    const incomeGroup = authedPage.locator('.category-group-card', {
      hasText: 'Income',
    });
    await incomeGroup.getByRole('button', { name: 'Create category' }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'New category' });
    await dialog.getByLabel('Name (English)').fill('Bonus Pay');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog).not.toBeVisible();

    const row = authedPage.locator('li.categories-list-row', {
      hasText: 'Bonus Pay',
    });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();
    const editDialog = authedPage.getByRole('dialog', {
      name: 'Edit category',
    });
    await expect(editDialog.getByLabel('Name (English)')).toHaveValue(
      'Bonus Pay',
    );
    await expect(editDialog.getByLabel('Name (French)')).toHaveValue(
      'Bonus Pay',
    );
  });

  test('creating a category with only a French name auto-fills the English name', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    const incomeGroup = authedPage.locator('.category-group-card', {
      hasText: 'Income',
    });
    await incomeGroup.getByRole('button', { name: 'Create category' }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'New category' });
    await dialog.getByLabel('Name (French)').fill('Prime');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog).not.toBeVisible();

    const row = authedPage.locator('li.categories-list-row', {
      hasText: 'Prime',
    });
    await row.getByRole('button', { name: 'Edit', exact: true }).click();
    const editDialog = authedPage.getByRole('dialog', {
      name: 'Edit category',
    });
    await expect(editDialog.getByLabel('Name (English)')).toHaveValue(
      'Prime',
    );
    await expect(editDialog.getByLabel('Name (French)')).toHaveValue(
      'Prime',
    );
  });

  test('leaving both names blank shows an inline error, not a toast', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    const incomeGroup = authedPage.locator('.category-group-card', {
      hasText: 'Income',
    });
    await incomeGroup.getByRole('button', { name: 'Create category' }).click();

    const dialog = authedPage.getByRole('dialog', { name: 'New category' });
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(dialog.getByRole('alert')).toHaveText('A name is required.');
    await expect(dialog).toBeVisible();
  });

  test('creating a group with only an English name auto-fills the French name', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    await authedPage
      .getByRole('button', { name: 'Create group', exact: true })
      .first()
      .click();

    const dialog = authedPage.getByRole('dialog', { name: 'New group' });
    await dialog.getByLabel('Name (English)').fill('Side Hustle');
    await dialog.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(dialog).not.toBeVisible();

    const card = authedPage.locator('.category-group-card', {
      hasText: 'Side Hustle',
    });
    await card
      .locator('.category-group-header')
      .getByRole('button', { name: 'Edit', exact: true })
      .click();
    const editDialog = authedPage.getByRole('dialog', { name: 'Edit group' });
    await expect(editDialog.getByLabel('Name (English)')).toHaveValue(
      'Side Hustle',
    );
    await expect(editDialog.getByLabel('Name (French)')).toHaveValue(
      'Side Hustle',
    );
  });

  test('drag-and-drop reorders categories within a group, and the new order survives a reload', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    const incomeGroup = authedPage.locator('.category-group-card', {
      hasText: 'Income',
    });
    const nameLocator = incomeGroup.locator('.categories-list-name');
    await expect(nameLocator.first()).toHaveText('Paychecks');

    const rows = incomeGroup.locator('li.categories-list-row');
    await rows
      .nth(0)
      .locator('.categories-list-drag-handle')
      .dragTo(rows.nth(2));

    const afterDrag = await nameLocator.allTextContents();
    expect(afterDrag[0]).not.toBe('Paychecks');
    expect(afterDrag).toContain('Paychecks');

    await authedPage.reload();
    await expect(
      authedPage
        .locator('.category-group-card', { hasText: 'Income' })
        .locator('.categories-list-name'),
    ).toHaveText(afterDrag);
  });

  test('move up/down buttons reorder categories within a group for anyone who cannot drag-and-drop, and survive a reload', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    const incomeGroup = authedPage.locator('.category-group-card', {
      hasText: 'Income',
    });
    const nameLocator = incomeGroup.locator('.categories-list-name');
    await expect(nameLocator.first()).toHaveText('Paychecks');

    const firstRow = incomeGroup.locator('li.categories-list-row').first();
    await expect(
      firstRow.getByRole('button', { name: 'Move Paychecks up' }),
    ).toBeDisabled();
    await firstRow
      .getByRole('button', { name: 'Move Paychecks down' })
      .click();

    await expect(nameLocator.first()).not.toHaveText('Paychecks');
    await expect(nameLocator.nth(1)).toHaveText('Paychecks');

    const afterMove = await nameLocator.allTextContents();
    await authedPage.reload();
    await expect(
      authedPage
        .locator('.category-group-card', { hasText: 'Income' })
        .locator('.categories-list-name'),
    ).toHaveText(afterMove);
  });

  test('move up/down buttons reorder groups within a type section for anyone who cannot drag-and-drop, and survive a reload', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/categories');
    const expensesSection = authedPage
      .locator('.category-type-section', { hasText: 'Expenses' })
      .first();
    const groupNameLocator = expensesSection.locator(
      '.category-group-header .category-group-name',
    );
    const firstGroupName = await groupNameLocator.first().textContent();

    const firstGroupHeader = expensesSection
      .locator('.category-group-header')
      .first();
    await expect(
      firstGroupHeader.getByRole('button', { name: /^Move .+ up$/ }),
    ).toBeDisabled();
    await firstGroupHeader
      .getByRole('button', { name: /^Move .+ down$/ })
      .click();

    await expect(groupNameLocator.first()).not.toHaveText(firstGroupName!);
    await expect(groupNameLocator.nth(1)).toHaveText(firstGroupName!);

    const afterMove = await groupNameLocator.allTextContents();
    await authedPage.reload();
    await expect(
      authedPage
        .locator('.category-type-section', { hasText: 'Expenses' })
        .first()
        .locator('.category-group-header .category-group-name'),
    ).toHaveText(afterMove);
  });
});
