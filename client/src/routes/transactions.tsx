import { createFileRoute, getRouteApi } from '@tanstack/react-router';
import { faFilter, faPlus } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../components/TopMenuButton';
import { TransactionsList } from '../features/transactions/TransactionsList';
import { SearchButton } from '../features/transactions/SearchButton';
import { DateRangeButton } from '../features/transactions/DateRangeButton';
import { isValidDateKey } from '../features/transactions/date-range';
import { DownloadButton } from '../features/transactions/DownloadButton';
import { ImportButton } from '../features/transactions/ImportButton';
import { notImplementedToast } from '../lib/toast';
import type { SortOrder } from '../features/transactions/types';
import '../components/TopMenu.css';

type TransactionsSearch = {
  search?: string;
  order?: SortOrder;
  start_date?: string;
  end_date?: string;
};

const SORT_ORDERS: SortOrder[] = [
  'date',
  'inverse_date',
  'amount',
  'inverse_amount',
];

const parseDateParam = (value: unknown): string | undefined =>
  typeof value === 'string' && isValidDateKey(value) ? value : undefined;

const routeApi = getRouteApi('/transactions');

const ClearAllButton = () => {
  const { search, order, start_date, end_date } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();

  if (!search && !order && !start_date && !end_date) return null;

  return (
    <button
      type="button"
      className="top-menu-clear-all"
      onClick={() => navigate({ search: {}, replace: true })}
    >
      Clear
    </button>
  );
};

const TransactionsTopMenuActions = () => (
  <>
    <ClearAllButton />
    <SearchButton />
    <DateRangeButton />
    <TopMenuButton
      icon={faFilter}
      label="Filters"
      onClick={notImplementedToast}
    />
    <ImportButton />
    <DownloadButton />
    <TopMenuButton
      icon={faPlus}
      label="Add"
      variant="primary"
      onClick={notImplementedToast}
    />
  </>
);

const Transactions = () => <TransactionsList />;

export const Route = createFileRoute('/transactions')({
  component: Transactions,
  validateSearch: (search: Record<string, unknown>): TransactionsSearch => ({
    search:
      typeof search.search === 'string' && search.search !== ''
        ? search.search
        : undefined,
    order: SORT_ORDERS.includes(search.order as SortOrder)
      ? (search.order as SortOrder)
      : undefined,
    start_date: parseDateParam(search.start_date),
    end_date: parseDateParam(search.end_date),
  }),
  staticData: {
    topMenuTitle: 'Transactions',
    topMenuActions: TransactionsTopMenuActions,
  },
});
