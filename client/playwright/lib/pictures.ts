import { expect, type Locator, type Page } from '@playwright/test';

/** A real PNG of this size, drawn in the browser, ready for `setInputFiles` or an API upload.
 * `fills` are equal vertical bands, left to right. */
export const pngFile = async (
  page: Page,
  width = 400,
  height = 200,
  fills = ['teal'],
) => {
  const dataUrl = await page.evaluate<string>(`(() => {
    const canvas = document.createElement('canvas');
    canvas.width = ${width};
    canvas.height = ${height};
    const context = canvas.getContext('2d');
    const fills = ${JSON.stringify(fills)};
    fills.forEach((fill, index) => {
      context.fillStyle = fill;
      context.fillRect((index * ${width}) / fills.length, 0, ${width} / fills.length, ${height});
    });
    return canvas.toDataURL('image/png');
  })()`);

  return {
    name: 'me.png',
    mimeType: 'image/png',
    buffer: Buffer.from(dataUrl.split(',')[1], 'base64'),
  };
};

/** Bytes that claim to be a PNG but aren't an image at all. */
export const notAnImageFile = {
  name: 'not-a-photo.png',
  mimeType: 'image/png',
  buffer: Buffer.from('this is not an image'),
};

/** The `[red, green, blue]` at the middle height of `image`'s left and right edges, once decoded. */
export const edgeColors = (page: Page, image: Buffer, contentType: string) =>
  page.evaluate<number[][]>(`(async () => {
    const bitmap = await createImageBitmap(
      await (await fetch('data:${contentType};base64,${image.toString('base64')}')).blob(),
    );
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext('2d');
    context.drawImage(bitmap, 0, 0);
    const y = Math.floor(bitmap.height / 2);
    return [10, bitmap.width - 10].map((x) =>
      Array.from(context.getImageData(x, y, 1, 1).data).slice(0, 3),
    );
  })()`);

/** Waits for the picture in `scope` to really load, not just for its element to appear. */
export const expectLoadedPicture = async (scope: Locator) => {
  const image = scope.locator('img.avatar');
  await expect(image).toBeVisible();
  await expect
    .poll(() =>
      image.evaluate((img: { naturalWidth: number }) => img.naturalWidth),
    )
    .toBeGreaterThan(0);
  return image;
};
