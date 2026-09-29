import {
  createFileRoute,
  getRouteApi,
  Outlet,
  useNavigate,
} from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { faFilter, faPlus } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../components/TopMenuButton';
import { TransactionsList } from '../features/transactions/components/TransactionsList';
import { SearchButton } from '../features/transactions/components/SearchButton';
import { DateRangeButton } from '../features/transactions/components/DateRangeButton';
import { isValidDateKey } from '../features/transactions/utils/date-range';
import { DownloadButton } from '../features/transactions/components/DownloadButton';
import { ImportButton } from '../features/transactions/components/ImportButton';
import { notImplementedToast } from '../lib/toast';
import type { SortOrder } from '../features/transactions/utils/types';
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

const routeApi = getRouteApi('/_app/transactions');

const ClearAllButton = () => {
  const { t } = useTranslation();
  const { search, order, start_date, end_date } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();

  if (!search && !order && !start_date && !end_date) return null;

  return (
    <button
      type="button"
      className="top-menu-clear-all"
      onClick={() => navigate({ search: {}, replace: true })}
    >
      {t('transactions.topMenu.clear')}
    </button>
  );
};

const TransactionsTopMenuActions = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <>
      <ClearAllButton />
      <SearchButton />
      <DateRangeButton />

      <TopMenuButton
        icon={faFilter}
        label={t('transactions.topMenu.filters')}
        onClick={notImplementedToast}
      />

      <ImportButton />
      <DownloadButton />

      <TopMenuButton
        id="add-transaction-button"
        icon={faPlus}
        label={t('transactions.topMenu.add')}
        variant="primary"
        aria-haspopup="dialog"
        onClick={() =>
          navigate({
            to: '/transactions/add-transaction',
            search: (prev) => prev,
          })
        }
      />
    </>
  );
};

const Transactions = () => (
  <>
    <TransactionsList />
    <Outlet />
  </>
);

export const Route = createFileRoute('/_app/transactions')({
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
    topMenuTitle: 'nav.transactions',
    topMenuActions: TransactionsTopMenuActions,
  },
});
