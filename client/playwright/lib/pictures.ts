import type { Page } from '@playwright/test';

/** A real PNG of this size, drawn in the browser, ready for `setInputFiles` or an API upload. */
export const pngFile = async (page: Page, width = 400, height = 200) => {
  const dataUrl = await page.evaluate<string>(`(() => {
    const canvas = document.createElement('canvas');
    canvas.width = ${width};
    canvas.height = ${height};
    const context = canvas.getContext('2d');
    context.fillStyle = 'teal';
    context.fillRect(0, 0, ${width}, ${height});
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
