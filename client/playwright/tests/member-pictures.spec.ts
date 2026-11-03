import { randomUUID } from 'node:crypto';
import type { APIRequestContext, Locator, Page } from '@playwright/test';
import { test, expect, type WorkerInfra } from '../fixtures';
import {
  edgeColors,
  expectLoadedPicture,
  notAnImageFile,
  pngFile,
} from '../lib/pictures';
import { ensureAccount } from '../lib/seed';

/** Has someone else (own cookie jar, via `request`) join `page`'s household. */
const addMember = async (
  page: Page,
  request: APIRequestContext,
  workerInfra: WorkerInfra,
  firstName = 'Jane',
) => {
  const me = await page.request.get(`${workerInfra.apiOrigin}/auth/me`);
  const { household } = await me.json();
  await request.post(`${workerInfra.apiOrigin}/auth/request-login`, {
    data: { email: `e2e-${randomUUID()}@example.test` },
  });
  await request.post(`${workerInfra.apiOrigin}/auth/onboarding`, {
    data: { first_name: firstName, join_code: household.join_code },
  });
};

const rowOf = (page: Page, name: string) =>
  page.locator('li.members-list-row', { hasText: name });

const openEditor = async (page: Page, name: string) => {
  await rowOf(page, name).getByRole('button', { name: 'Edit' }).click();
  return page.getByRole('dialog');
};

const choosePhoto = (
  scope: Page | Locator,
  file: Awaited<ReturnType<typeof pngFile>>,
) => scope.getByLabel('Choose photo').setInputFiles(file);

const expectInitial = async (scope: Locator, initial: string) => {
  await expect(scope.locator('img.avatar')).toHaveCount(0);
  await expect(scope.locator('span.avatar')).toHaveText(initial);
};

test.describe('Member pictures', () => {
  test('a member without a picture shows their initial', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');

    await expectInitial(rowOf(authedPage, 'Test User'), 'T');
    await expectInitial(authedPage.locator('.left-menu-profile'), 'T');
  });

  test('a chosen photo shows in the editor, the list, and the left menu, and survives a reload', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');
    await expect(dialog).toContainText('Test User');
    await expect(
      dialog.getByRole('button', { name: 'Remove photo' }),
    ).toHaveCount(0);

    await choosePhoto(dialog, await pngFile(authedPage));

    await expectLoadedPicture(dialog);
    await expect(
      dialog.getByRole('button', { name: 'Remove photo' }),
    ).toBeVisible();
    await expectLoadedPicture(rowOf(authedPage, 'Test User'));
    await expectLoadedPicture(authedPage.locator('.left-menu-profile'));

    await authedPage.reload();
    await expectLoadedPicture(rowOf(authedPage, 'Test User'));
    await expectLoadedPicture(authedPage.locator('.left-menu-profile'));
  });

  test('a photo is cropped to a small square JPEG before it is uploaded', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');

    // Only the middle band is inside a centered square, so stretching would show all three.
    await choosePhoto(
      dialog,
      await pngFile(authedPage, 1200, 300, ['red', 'lime', 'blue']),
    );

    const image = await expectLoadedPicture(dialog);
    const size = await image.evaluate(
      (img: { naturalWidth: number; naturalHeight: number }) => [
        img.naturalWidth,
        img.naturalHeight,
      ],
    );
    expect(size).toEqual([256, 256]);

    const response = await authedPage.request.get(
      (await image.getAttribute('src'))!,
    );
    expect(response.headers()['content-type']).toBe('image/jpeg');
    expect(response.headers()['x-content-type-options']).toBe('nosniff');

    const colors = await edgeColors(
      authedPage,
      await response.body(),
      'image/jpeg',
    );
    for (const [red, green, blue] of colors) {
      expect(green).toBeGreaterThan(200);
      expect(red).toBeLessThan(60);
      expect(blue).toBeLessThan(60);
    }
  });

  test('choosing another photo replaces the first', async ({ authedPage }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');

    await choosePhoto(dialog, await pngFile(authedPage));
    const image = await expectLoadedPicture(dialog);
    const firstSrc = await image.getAttribute('src');
    const firstBytes = await (await authedPage.request.get(firstSrc!)).body();

    await choosePhoto(dialog, await pngFile(authedPage, 300, 300, ['orange']));

    await expect(image).not.toHaveAttribute('src', firstSrc!);
    await expectLoadedPicture(dialog);
    const secondBytes = await (
      await authedPage.request.get((await image.getAttribute('src'))!)
    ).body();
    expect(secondBytes.equals(firstBytes)).toBe(false);
  });

  test('removing the photo brings the initial back everywhere', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');
    await choosePhoto(dialog, await pngFile(authedPage));
    await expectLoadedPicture(dialog);

    await dialog.getByRole('button', { name: 'Remove photo' }).click();

    await expectInitial(dialog, 'T');
    await expect(
      dialog.getByRole('button', { name: 'Remove photo' }),
    ).toHaveCount(0);
    await expectInitial(rowOf(authedPage, 'Test User'), 'T');
    await expectInitial(authedPage.locator('.left-menu-profile'), 'T');

    await authedPage.reload();
    await expectInitial(rowOf(authedPage, 'Test User'), 'T');
  });

  test('says so when the file is not an image', async ({ authedPage }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');

    await dialog.getByLabel('Choose photo').setInputFiles(notAnImageFile);

    await expect(
      authedPage.getByText(
        'This photo could not be read. Try a PNG, JPEG, or WebP image.',
      ),
    ).toBeVisible();
    await expectInitial(dialog, 'T');
  });

  test('says so when the server refuses the photo', async ({ authedPage }) => {
    await authedPage.route('**/members/*/picture', (route) =>
      route.fulfill({ status: 400, json: { error: 'invalid' } }),
    );
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');

    await choosePhoto(dialog, await pngFile(authedPage));

    await expect(
      authedPage.getByText(
        'This photo could not be read. Try a PNG, JPEG, or WebP image.',
      ),
    ).toBeVisible();
  });

  test('a failed upload can be tried again with the same file', async ({
    authedPage,
  }) => {
    await authedPage.route(
      '**/members/*/picture',
      (route) => route.fulfill({ status: 500, json: { error: 'down' } }),
      { times: 1 },
    );
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');
    const file = await pngFile(authedPage);

    await choosePhoto(dialog, file);
    await expect(
      authedPage.getByText('Could not save the photo. Please try again.'),
    ).toBeVisible();
    await expectInitial(dialog, 'T');

    await choosePhoto(dialog, file);
    await expectLoadedPicture(dialog);
  });

  test('a failed removal keeps the photo', async ({ authedPage }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');
    await choosePhoto(dialog, await pngFile(authedPage));
    await expectLoadedPicture(dialog);

    await authedPage.route('**/members/*/picture', (route) =>
      route.request().method() === 'DELETE'
        ? route.fulfill({ status: 500, json: { error: 'down' } })
        : route.continue(),
    );
    await dialog.getByRole('button', { name: 'Remove photo' }).click();

    await expect(
      authedPage.getByText('Could not remove the photo. Please try again.'),
    ).toBeVisible();
    await expectLoadedPicture(dialog);
  });

  test('the buttons wait while a photo is uploading', async ({
    authedPage,
  }) => {
    let finishUpload = () => {};
    const uploadCanFinish = new Promise<void>((resolve) => {
      finishUpload = resolve;
    });
    await authedPage.route('**/members/*/picture', async (route) => {
      if (route.request().method() === 'PUT') await uploadCanFinish;
      await route.continue();
    });
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');
    const choose = dialog.getByRole('button', { name: 'Choose photo' });

    await choosePhoto(dialog, await pngFile(authedPage));
    await expect(choose).toBeDisabled();

    // Still waiting after closing the editor and opening it again.
    await authedPage.keyboard.press('Escape');
    const reopened = await openEditor(authedPage, 'Test User');
    await expect(
      reopened.getByRole('button', { name: 'Choose photo' }),
    ).toBeDisabled();

    // Tab stays inside the editor: its only button that can take focus right now is Close.
    const close = reopened.getByRole('button', { name: 'Close' });
    await expect(close).toBeFocused();
    await authedPage.keyboard.press('Tab');
    await expect(close).toBeFocused();

    finishUpload();
    await expect(
      reopened.getByRole('button', { name: 'Choose photo' }),
    ).toBeEnabled();
    await expectLoadedPicture(dialog);
  });

  test('a picture that fails to load falls back to the initial', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');
    const dialog = await openEditor(authedPage, 'Test User');
    await choosePhoto(dialog, await pngFile(authedPage));
    await expectLoadedPicture(dialog);

    await authedPage.route('**/members/*/picture?*', (route) => route.abort());
    await authedPage.reload();

    await expectInitial(rowOf(authedPage, 'Test User'), 'T');
    await expectInitial(authedPage.locator('.left-menu-profile'), 'T');
  });

  test('the editor closes with its X, Escape, or a click outside, and focus returns to Edit', async ({
    authedPage,
  }) => {
    await authedPage.goto('/settings/members');
    const edit = rowOf(authedPage, 'Test User').getByRole('button', {
      name: 'Edit',
    });

    let dialog = await openEditor(authedPage, 'Test User');
    await dialog.getByRole('button', { name: 'Close' }).click();
    await expect(dialog).toHaveCount(0);
    await expect(edit).toBeFocused();

    dialog = await openEditor(authedPage, 'Test User');
    await authedPage.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);
    await expect(edit).toBeFocused();

    dialog = await openEditor(authedPage, 'Test User');
    await authedPage.mouse.click(5, 5);
    await expect(dialog).toHaveCount(0);
    await expect(edit).toBeFocused();
  });

  test('a manager can change the picture of anyone in the household', async ({
    authedPage,
    request,
    workerInfra,
  }) => {
    await addMember(authedPage, request, workerInfra);
    await authedPage.goto('/settings/members');

    const dialog = await openEditor(authedPage, 'Jane');
    await expect(dialog).toContainText('Jane');
    await choosePhoto(dialog, await pngFile(authedPage));
    await expectLoadedPicture(dialog);

    await expectLoadedPicture(rowOf(authedPage, 'Jane'));
    // Someone else's picture isn't the manager's own.
    await expectInitial(rowOf(authedPage, 'Test User'), 'T');
    await expectInitial(authedPage.locator('.left-menu-profile'), 'T');

    await dialog.getByRole('button', { name: 'Remove photo' }).click();
    await expectInitial(rowOf(authedPage, 'Jane'), 'J');
  });

  test('a member can edit only their own row, and sees everyone else’s picture', async ({
    authedPage,
    request,
    workerInfra,
  }) => {
    // Here `request` is the manager, and `authedPage` becomes the member who joins them.
    await request.post(`${workerInfra.apiOrigin}/auth/request-login`, {
      data: { email: `e2e-${randomUUID()}@example.test` },
    });
    const onboarded = await request.post(
      `${workerInfra.apiOrigin}/auth/onboarding`,
      { data: { first_name: 'Marie', last_name: 'Manager', join_code: null } },
    );
    const manager = await onboarded.json();
    await authedPage.goto('/settings/members');
    await request.put(
      `${workerInfra.apiOrigin}/members/${manager.user.id}/picture`,
      { multipart: { file: await pngFile(authedPage) } },
    );

    await authedPage.context().clearCookies();
    await authedPage.request.post(
      `${workerInfra.apiOrigin}/auth/request-login`,
      { data: { email: `e2e-${randomUUID()}@example.test` } },
    );
    await authedPage.request.post(`${workerInfra.apiOrigin}/auth/onboarding`, {
      data: {
        first_name: 'Jane',
        join_code: manager.household.join_code,
      },
    });
    await authedPage.goto('/settings/members');

    await expectLoadedPicture(rowOf(authedPage, 'Marie Manager'));
    await expect(
      rowOf(authedPage, 'Marie Manager').getByRole('button', { name: 'Edit' }),
    ).toHaveCount(0);

    const dialog = await openEditor(authedPage, 'Jane');
    await choosePhoto(dialog, await pngFile(authedPage));
    await expectLoadedPicture(dialog);
    await expectLoadedPicture(authedPage.locator('.left-menu-profile'));
  });

  test('is translated in French', async ({ authedPage }) => {
    await authedPage.addInitScript(() =>
      localStorage.setItem('firebird-language', 'fr'),
    );
    await authedPage.goto('/settings/members');

    await rowOf(authedPage, 'Test User')
      .getByRole('button', { name: 'Modifier' })
      .click();
    const dialog = authedPage.getByRole('dialog');
    await dialog.getByLabel('Choisir une photo').setInputFiles(notAnImageFile);

    await expect(
      authedPage.getByText(
        'Impossible de lire cette photo. Essaie une image PNG, JPEG ou WebP.',
      ),
    ).toBeVisible();
    await dialog
      .getByLabel('Choisir une photo')
      .setInputFiles(await pngFile(authedPage));
    await expect(
      dialog.getByRole('button', { name: 'Retirer la photo' }),
    ).toBeVisible();
  });

  test('the Filters popover shows the same avatar for what has no picture', async ({
    authedPage,
    workerInfra,
  }) => {
    await ensureAccount(authedPage.request, workerInfra.apiOrigin, 'Chequing');
    await authedPage.goto('/transactions');

    await authedPage.getByRole('button', { name: 'Filters' }).click();
    await authedPage.getByRole('button', { name: 'Accounts' }).click();

    await expect(
      authedPage
        .locator('.filters-popover-row', { hasText: 'Chequing' })
        .locator('span.avatar'),
    ).toHaveText('S');
  });
});
