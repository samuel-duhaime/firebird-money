import type { SortOrder } from './types';

export type AmountOp = 'greater_than' | 'less_than' | 'equal_to' | 'between';

/** Amount operators offered in the Filters popover, in display order. */
export const AMOUNT_OPS: AmountOp[] = [
  'greater_than',
  'less_than',
  'equal_to',
  'between',
];

/** The Filters popover's own fields — each one also a `/transactions` URL search param. */
export type TransactionFilters = {
  category_ids?: number[];
  merchant_ids?: number[];
  account_ids?: number[];
  tag_ids?: number[];
  /** Also match transactions without any tag (OR-ed with `tag_ids`). */
  untagged?: boolean;
  amount_op?: AmountOp;
  amount?: number;
  /** Upper bound, only meaningful when `amount_op` is `between`. */
  amount_max?: number;
};

/** Everything that narrows/sorts the transactions list, as carried in the page's URL. */
export type TransactionsQuery = TransactionFilters & {
  search?: string;
  order?: SortOrder;
  start_date?: string;
  end_date?: string;
};

export const FILTER_KEYS = [
  'category_ids',
  'merchant_ids',
  'account_ids',
  'tag_ids',
  'untagged',
  'amount_op',
  'amount',
  'amount_max',
] as const satisfies readonly (keyof TransactionFilters)[];

/** Every Filters popover key set to `undefined`, for clearing them all out of the URL at once. */
export const EMPTY_FILTERS: TransactionFilters = Object.fromEntries(
  FILTER_KEYS.map((key) => [key, undefined]),
);

/** How many filters are active — one per selected id, plus one each for "untagged" and amount. */
export const countFilters = (filters: TransactionFilters): number =>
  (filters.category_ids?.length ?? 0) +
  (filters.merchant_ids?.length ?? 0) +
  (filters.account_ids?.length ?? 0) +
  (filters.tag_ids?.length ?? 0) +
  (filters.untagged ? 1 : 0) +
  (filters.amount_op && filters.amount !== undefined ? 1 : 0);

// Each parser takes both shapes a value can arrive in: text read back from the URL (see
// `lib/search-params.ts`), e.g. `"15,3"`, and the typed value a `navigate()` call passes in,
// e.g. `[15, 3]` — the router runs `validateSearch` on both.

const parseIdList = (value: unknown): number[] | undefined => {
  const raw =
    typeof value === 'string'
      ? value.split(',')
      : Array.isArray(value)
        ? value
        : [];
  const ids = raw
    .map((id) => (typeof id === 'string' && /^\d+$/.test(id) ? Number(id) : id))
    .filter(
      (id): id is number => typeof id === 'number' && Number.isInteger(id),
    );
  return ids.length > 0 ? ids : undefined;
};

const parseAmount = (value: unknown): number | undefined => {
  const amount =
    typeof value === 'string' && /^\d+(\.\d+)?$/.test(value)
      ? Number(value)
      : value;
  return typeof amount === 'number' && Number.isFinite(amount) && amount >= 0
    ? amount
    : undefined;
};

/** Validates the Filters popover's URL search params, dropping anything malformed. */
export const parseFilters = (
  search: Record<string, unknown>,
): TransactionFilters => {
  const amountOp = AMOUNT_OPS.includes(search.amount_op as AmountOp)
    ? (search.amount_op as AmountOp)
    : undefined;
  const amount = amountOp ? parseAmount(search.amount) : undefined;

  return {
    category_ids: parseIdList(search.category_ids),
    merchant_ids: parseIdList(search.merchant_ids),
    account_ids: parseIdList(search.account_ids),
    tag_ids: parseIdList(search.tag_ids),
    untagged:
      search.untagged === true || search.untagged === 'true' ? true : undefined,
    amount_op: amount !== undefined ? amountOp : undefined,
    amount,
    amount_max:
      amountOp === 'between' && amount !== undefined
        ? parseAmount(search.amount_max)
        : undefined,
  };
};

/** Builds the `GET /transactions` (and `/transactions/download`) query params for `query`. */
export const transactionsQueryParams = (
  query: TransactionsQuery,
): URLSearchParams => {
  const params = new URLSearchParams();
  if (query.search) params.set('search', query.search);
  if (query.order) params.set('order', query.order);
  if (query.start_date) params.set('start_date', query.start_date);
  if (query.end_date) params.set('end_date', query.end_date);
  for (const key of [
    'category_ids',
    'merchant_ids',
    'account_ids',
    'tag_ids',
  ] as const) {
    const ids = query[key];
    if (ids?.length) params.set(key, ids.join(','));
  }
  if (query.untagged) params.set('untagged', 'true');
  if (query.amount_op && query.amount !== undefined) {
    params.set('amount_op', query.amount_op);
    params.set('amount', String(query.amount));
    if (query.amount_op === 'between' && query.amount_max !== undefined) {
      params.set('amount_max', String(query.amount_max));
    }
  }
  return params;
};
