import { createFileRoute } from '@tanstack/react-router';
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

type TransactionsSearch = { search?: string };

const TransactionsTopMenuActions = () => (
  <>
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
  }),
  staticData: { topMenuTitle: 'Transactions', topMenuActions: TransactionsTopMenuActions },
});
