/**
 * The router's URL search-param format, replacing TanStack Router's default JSON one (which turns
 * `[15, 3]` into `%5B15%2C3%5D` and a string like `"123"` into `%22123%22`). Values are written as
 * plain text — arrays comma-joined, e.g. `?tag_ids=15,3` — and always read back as strings, so
 * each route's `validateSearch` is the one place that turns them into numbers, lists, etc.
 */

/** Percent-encodes `value`, but leaves commas readable since they're only list separators here. */
const encode = (value: string): string =>
  encodeURIComponent(value).replace(/%2C/gi, ',');

export const stringifySearch = (search: Record<string, unknown>): string => {
  const parts = Object.entries(search).flatMap(([key, value]) => {
    if (value === undefined || value === null || value === '') return [];
    const text = Array.isArray(value) ? value.join(',') : String(value);
    return text === '' ? [] : [`${encode(key)}=${encode(text)}`];
  });
  return parts.length > 0 ? `?${parts.join('&')}` : '';
};

export const parseSearch = (searchString: string): Record<string, string> =>
  Object.fromEntries(new URLSearchParams(searchString));
