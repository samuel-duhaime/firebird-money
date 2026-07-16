import { createFileRoute } from '@tanstack/react-router';
import {
  faMagnifyingGlass,
  faCalendarDays,
  faFilter,
  faRuler,
  faDownload,
  faPlus,
} from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../components/TopMenuButton';

const TransactionsTopMenuActions = () => (
  <>
    <TopMenuButton icon={faMagnifyingGlass} label="Search" />
    <TopMenuButton icon={faCalendarDays} label="Date" />
    <TopMenuButton icon={faFilter} label="Filters" />
    <TopMenuButton icon={faRuler} label="Rules" />
    <TopMenuButton icon={faDownload} label="Download" />
    <TopMenuButton icon={faPlus} label="Add" variant="primary" />
  </>
);

const Transactions = () => null;

export const Route = createFileRoute('/transactions')({
  component: Transactions,
  staticData: { topMenuTitle: 'Transactions', topMenuActions: TransactionsTopMenuActions },
});
