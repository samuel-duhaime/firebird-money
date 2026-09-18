/**
 * Strips everything except digits and the two accepted decimal separators, so a typed or pasted
 * value can never carry a sign — amounts are always stored positive, with income vs. expense
 * coming from the transaction's category (see `categories` table). Meant to run on every
 * keystroke, upstream of `normalizeAmount`.
 */
export const sanitizeAmountInput = (rawAmount: string): string =>
  rawAmount.replace(/[^0-9.,]/g, '');

/**
 * Why `normalizeAmount` rejected a value — lets callers show a message that explains which rule
 * was broken, rather than one generic "invalid amount" for every case.
 *
 * - `ambiguous`: not a plain decimal number at all (a sign, letters, two separators, a separator
 *   with no digits, or a single separator read as thousands-grouping).
 * - `tooLong`: a plain, unambiguous number, but with more whole-number digits than the server's
 *   `NUMERIC(12, 2)` column can store (max 10, since 2 are reserved for the fraction).
 */
export type AmountError = 'ambiguous' | 'tooLong';

export type NormalizeAmountResult =
  { valid: true; value: string } | { valid: false; error: AmountError };

/**
 * The amount input accepts a comma or a period as the decimal separator (French keyboards
 * produce a comma), but a value with both — e.g. "1,234.56" — or a single separator followed by
 * 3+ digits — e.g. "1,234" — reads as thousands-grouped. Guessing which separator was meant would
 * silently change the amount rather than fail loudly, so both are rejected instead. Returns the
 * amount as a server-compatible decimal string ("1234.56"), or the reason it's invalid.
 */
export const normalizeAmount = (rawAmount: string): NormalizeAmountResult => {
  // Amounts are always stored positive — direction comes from the transaction's category (see
  // the `categories` table) — so a sign (or any other stray character) makes the value invalid
  // rather than something to guess a meaning for.
  if (/[^0-9.,]/.test(rawAmount)) return { valid: false, error: 'ambiguous' };
  // Rejects "" and "." — a separator (or nothing) with no digits at all isn't an amount, even
  // though neither trips the checks below.
  if (!/[0-9]/.test(rawAmount)) return { valid: false, error: 'ambiguous' };

  const separators = rawAmount.match(/[.,]/g) ?? [];
  if (separators.length > 1) return { valid: false, error: 'ambiguous' };

  const [wholePart, fractionPart] = rawAmount.split(/[.,]/);
  if (fractionPart !== undefined && fractionPart.length > 2) {
    return { valid: false, error: 'ambiguous' };
  }
  // The server stores amounts as NUMERIC(12, 2) — 12 significant digits, 2 of them after the
  // separator — so more than 10 digits ahead of it can never be saved.
  if (wholePart.length > 10) return { valid: false, error: 'tooLong' };

  return {
    valid: true,
    value:
      fractionPart === undefined ? wholePart : `${wholePart}.${fractionPart}`,
  };
};
