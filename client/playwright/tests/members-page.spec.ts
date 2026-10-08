import { test, expect } from '../fixtures';

test.describe('Household > Members settings', () => {
  test('left nav links to Household, and lands on the Members page', async ({
    authedPage,
  }) => {
    await authedPage.goto('/dashboard');
    await authedPage
      .getByRole('link', { name: 'Household', exact: true })
      .click();

    await expect(authedPage).toHaveURL('/settings/members');
    await expect(authedPage.locator('.top-menu-title')).toHaveText('Settings');
    await expect(
      authedPage.getByRole('link', { name: 'Members', exact: true }),
    ).toHaveClass(/settings-nav-link--active/);
  });

  test('lists the signed-in member with their name, email, and status', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');

    const row = authedPage.locator('li.members-list-row');
    await expect(row).toHaveCount(1);
    await expect(row).toContainText('Test User');
    await expect(row).toContainText('Manager');
    await expect(row).toContainText('@example.test');
    // The test server signs in without an email, so nobody is ever verified.
    await expect(row).toContainText('Pending');
  });

  test('shows the invite code and copies it', async ({
    authedPage,
    context,
  }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await authedPage.goto('/settings/members');

    const code = authedPage.locator('.invite-code-value');
    await expect(code).toHaveText(/^[0-9A-F]{8}$/);

    await authedPage.getByRole('button', { name: 'Copy', exact: true }).click();

    await expect(authedPage.getByText('Invite code copied.')).toBeVisible();
    expect(
      await authedPage.evaluate<string>('navigator.clipboard.readText()'),
    ).toBe(await code.textContent());
  });
});
