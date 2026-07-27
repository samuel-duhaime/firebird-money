export type DateRangePreset =
  | 'last_7_days'
  | 'last_30_days'
  | 'this_month'
  | 'last_month'
  | 'this_year'
  | 'last_year';

/** Matches a `YYYY-MM-DD` date key, the format used both by the API and this popover's inputs. */
export const DATE_KEY_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

export const isValidDateKey = (value: string): boolean =>
  DATE_KEY_PATTERN.test(value);

/** Presets offered in the date-range popover, in display order. */
export const DATE_RANGE_PRESETS: { value: DateRangePreset; label: string }[] = [
  { value: 'last_7_days', label: 'Last 7 days' },
  { value: 'last_30_days', label: 'Last 30 days' },
  { value: 'this_month', label: 'This month' },
  { value: 'last_month', label: 'Last month' },
  { value: 'this_year', label: 'This year' },
  { value: 'last_year', label: 'Last year' },
];

/** Formats a `Date` as `YYYY-MM-DD` using local date components, matching the API's date format. */
const toDateKey = (date: Date): string => {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
};

/** Parses a `YYYY-MM-DD` string into a local `Date`, without UTC-shifting the day. */
const parseDateKey = (dateKey: string): Date => {
  const [year, month, day] = dateKey.split('-').map(Number);
  return new Date(year, month - 1, day);
};

/** Resolves a preset into an inclusive `[start_date, end_date]` range, relative to `today`. */
export const resolvePreset = (
  preset: DateRangePreset,
  today: Date = new Date(),
): { start_date: string; end_date: string } => {
  const daysAgo = (days: number) =>
    new Date(today.getFullYear(), today.getMonth(), today.getDate() - days);

  switch (preset) {
    case 'last_7_days':
      return { start_date: toDateKey(daysAgo(6)), end_date: toDateKey(today) };
    case 'last_30_days':
      return { start_date: toDateKey(daysAgo(29)), end_date: toDateKey(today) };
    case 'this_month':
      return {
        start_date: toDateKey(
          new Date(today.getFullYear(), today.getMonth(), 1),
        ),
        end_date: toDateKey(today),
      };
    case 'last_month':
      return {
        start_date: toDateKey(
          new Date(today.getFullYear(), today.getMonth() - 1, 1),
        ),
        end_date: toDateKey(new Date(today.getFullYear(), today.getMonth(), 0)),
      };
    case 'this_year':
      return {
        start_date: toDateKey(new Date(today.getFullYear(), 0, 1)),
        end_date: toDateKey(today),
      };
    case 'last_year':
      return {
        start_date: toDateKey(new Date(today.getFullYear() - 1, 0, 1)),
        end_date: toDateKey(new Date(today.getFullYear() - 1, 11, 31)),
      };
  }
};

/** Formats a `start_date`/`end_date` pair as a compact label, e.g. `Jul 21 – Jul 27`. */
export const formatDateRangeLabel = (
  startDate?: string,
  endDate?: string,
): string => {
  const format = (dateKey: string) =>
    parseDateKey(dateKey).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    });

  if (startDate && endDate) {
    return startDate === endDate
      ? format(startDate)
      : `${format(startDate)} – ${format(endDate)}`;
  }
  if (startDate) return `From ${format(startDate)}`;
  if (endDate) return `Until ${format(endDate)}`;
  return 'Date';
};
