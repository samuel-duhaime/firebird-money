import { createFileRoute, getRouteApi } from '@tanstack/react-router';
import {
  faCalendarDays,
  faFilter,
  faFileImport,
  faDownload,
  faPlus,
} from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../components/TopMenuButton';
import { TransactionsList } from '../features/transactions/TransactionsList';
import { SearchButton } from '../features/transactions/SearchButton';
import { notImplementedToast } from '../lib/toast';
import type { SortOrder } from '../features/transactions/types';
import '../components/TopMenu.css';

type TransactionsSearch = { search?: string; order?: SortOrder };

const SORT_ORDERS: SortOrder[] = ['date', 'inverse_date', 'amount', 'inverse_amount'];

const routeApi = getRouteApi('/transactions');

const ClearAllButton = () => {
  const { search, order } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();

  if (!search && !order) return null;

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
    <TopMenuButton icon={faCalendarDays} label="Date" onClick={notImplementedToast} />
    <TopMenuButton icon={faFilter} label="Filters" onClick={notImplementedToast} />
    <TopMenuButton icon={faFileImport} label="Import" onClick={notImplementedToast} />
    <TopMenuButton icon={faDownload} label="Download" onClick={notImplementedToast} />
    <TopMenuButton icon={faPlus} label="Add" variant="primary" onClick={notImplementedToast} />
  </>
);

const Transactions = () => <TransactionsList />;

export const Route = createFileRoute('/transactions')({
  component: Transactions,
  validateSearch: (search: Record<string, unknown>): TransactionsSearch => ({
    search: typeof search.search === 'string' && search.search !== '' ? search.search : undefined,
    order: SORT_ORDERS.includes(search.order as SortOrder) ? (search.order as SortOrder) : undefined,
  }),
  staticData: { topMenuTitle: 'Transactions', topMenuActions: TransactionsTopMenuActions },
});
