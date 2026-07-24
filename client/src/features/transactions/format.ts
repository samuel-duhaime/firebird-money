/** Formats a signed number of dollars as `$1,234.56` (or `-$1,234.56` if negative). */
export const formatAmount = (amount: number): string => {
  const sign = amount < 0 ? '-' : '';
  return `${sign}$${Math.abs(amount).toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
};

/** Formats an ISO `YYYY-MM-DD` date as `October 14, 2025`, without UTC-shifting the day. */
export const formatDateHeading = (isoDate: string): string => {
  const [year, month, day] = isoDate.split('-').map(Number);
  return new Date(year, month - 1, day).toLocaleDateString('en-US', {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
  });
};
