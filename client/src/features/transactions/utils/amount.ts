/**
 * Strips everything except digits and the two accepted decimal separators, so a typed or pasted
 * value can never carry a sign — amounts are always stored positive, with income vs. expense
 * coming from the transaction's category (see `categories` table). Meant to run on every
 * keystroke, upstream of `normalizeAmount`.
 */
export const sanitizeAmountInput = (rawAmount: string): string =>
  rawAmount.replace(/[^0-9.,]/g, '');

/**
 * The amount input accepts a comma or a period as the decimal separator (French keyboards
 * produce a comma), but a value with both — e.g. "1,234.56" — or a single separator followed by
 * 3+ digits — e.g. "1,234" — reads as thousands-grouped. Guessing which separator was meant would
 * silently change the amount rather than fail loudly, so both are rejected instead. Returns the
 * amount as a server-compatible decimal string ("1234.56"), or null if it's ambiguous.
 */
export const normalizeAmount = (rawAmount: string): string | null => {
  // Amounts are always stored positive — direction comes from the transaction's category (see
  // the `categories` table) — so a sign (or any other stray character) makes the value invalid
  // rather than something to guess a meaning for.
  if (/[^0-9.,]/.test(rawAmount)) return null;

  const separators = rawAmount.match(/[.,]/g) ?? [];
  if (separators.length > 1) return null;

  const [wholePart, fractionPart] = rawAmount.split(/[.,]/);
  if (fractionPart !== undefined && fractionPart.length > 2) return null;

  return fractionPart === undefined
    ? wholePart
    : `${wholePart}.${fractionPart}`;
};
