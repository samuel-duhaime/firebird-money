/** Formats a signed number of US dollars, e.g. `$1,234.56` (or its locale equivalent). */
export const formatAmount = (amount: number, locale: string): string =>
  new Intl.NumberFormat(locale, { style: 'currency', currency: 'USD' }).format(amount);

/** Formats an ISO `YYYY-MM-DD` date as `October 14, 2025`, without UTC-shifting the day. */
export const formatDateHeading = (isoDate: string, locale: string): string => {
  const [year, month, day] = isoDate.split('-').map(Number);
  return new Date(year, month - 1, day).toLocaleDateString(locale, {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
  });
};
