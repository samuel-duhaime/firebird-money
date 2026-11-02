/** Width and height of every stored picture: sharp at the largest size an avatar shows at. */
const PICTURE_SIZE = 256;

/** The browser can't read the chosen file as an image. */
export class UnreadablePictureError extends Error {}

/**
 * Crops `file` to a centered square and shrinks it, so a large phone photo uploads in a moment and
 * stays small in the database. JPEG, since every browser can encode it; transparency turns white.
 */
export const resizePicture = async (file: File): Promise<Blob> => {
  let image: ImageBitmap;
  try {
    image = await createImageBitmap(file);
  } catch {
    throw new UnreadablePictureError();
  }

  const side = Math.min(image.width, image.height);
  const canvas = document.createElement('canvas');
  canvas.width = PICTURE_SIZE;
  canvas.height = PICTURE_SIZE;
  const context = canvas.getContext('2d');
  if (!context) throw new UnreadablePictureError();
  context.fillStyle = 'white';
  context.fillRect(0, 0, PICTURE_SIZE, PICTURE_SIZE);
  context.drawImage(
    image,
    (image.width - side) / 2,
    (image.height - side) / 2,
    side,
    side,
    0,
    0,
    PICTURE_SIZE,
    PICTURE_SIZE,
  );
  image.close();

  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new UnreadablePictureError())),
      'image/jpeg',
      0.85,
    );
  });
};
