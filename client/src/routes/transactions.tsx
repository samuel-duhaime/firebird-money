import { createFileRoute } from '@tanstack/react-router';
import {
  faMagnifyingGlass,
  faCalendarDays,
  faFilter,
  faFileImport,
  faDownload,
  faPlus,
} from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../components/TopMenuButton';
import { TransactionsList } from '../features/transactions/TransactionsList';
import { notImplementedToast } from '../lib/toast';

const TransactionsTopMenuActions = () => (
  <>
    <TopMenuButton icon={faMagnifyingGlass} label="Search" onClick={notImplementedToast} />
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
  staticData: { topMenuTitle: 'Transactions', topMenuActions: TransactionsTopMenuActions },
});
