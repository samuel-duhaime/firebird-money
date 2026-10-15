import {
  createFileRoute,
  getRouteApi,
  Outlet,
  useNavigate,
} from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { faPlus } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../components/TopMenuButton';
import { TransactionsList } from '../features/transactions/components/TransactionsList';
import { SearchButton } from '../features/transactions/components/SearchButton';
import { DateRangeButton } from '../features/transactions/components/DateRangeButton';
import { isValidDateKey } from '../features/transactions/utils/date-range';
import { DownloadButton } from '../features/transactions/components/DownloadButton';
import { ImportButton } from '../features/transactions/components/ImportButton';
import { FiltersButton } from '../features/transactions/components/FiltersButton';
import {
  countFilters,
  parseFilters,
} from '../features/transactions/utils/filters';
import type { TransactionsQuery } from '../features/transactions/utils/filters';
import type { SortOrder } from '../features/transactions/utils/types';
import '../components/TopMenu.css';

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
  const query = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const { search, order, start_date, end_date } = query;

  if (!search && !order && !start_date && !end_date && countFilters(query) === 0)
    return null;

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

      <FiltersButton />

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
  validateSearch: (search: Record<string, unknown>): TransactionsQuery => ({
    search:
      typeof search.search === 'string' && search.search !== ''
        ? search.search
        : undefined,
    order: SORT_ORDERS.includes(search.order as SortOrder)
      ? (search.order as SortOrder)
      : undefined,
    start_date: parseDateParam(search.start_date),
    end_date: parseDateParam(search.end_date),
    ...parseFilters(search),
  }),
  staticData: {
    topMenuTitle: 'nav.transactions',
    topMenuActions: TransactionsTopMenuActions,
  },
});
