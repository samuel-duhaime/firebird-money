import type { ReactNode } from 'react';
import { Link } from '@tanstack/react-router';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faArrowRight } from '@fortawesome/free-solid-svg-icons';
import type { TransactionFilters } from '../utils/filters';
import './ViewTransactionsLink.css';

/** The Filters popover param that narrows the transactions list to one merchant, category, ... */
export type ViewTransactionsFilter =
  'category_ids' | 'merchant_ids' | 'account_ids' | 'tag_ids';

type ViewTransactionsProps = {
  filter: ViewTransactionsFilter;
  id: number;
};

/** `/transactions` narrowed to just `id` — any other active search/filter is dropped, so the list
 * shows every transaction of that one merchant/category/account/tag. */
const searchFor = ({
  filter,
  id,
}: ViewTransactionsProps): TransactionFilters => ({
  [filter]: [id],
});

/** The square arrow next to a transactions-row cell, e.g. "View merchant". Hidden until its cell
 * group is hovered or focused (see `.transactions-row-cell-group`). */
export const ViewTransactionsArrow = ({
  label,
  ...props
}: ViewTransactionsProps & { label: string }) => (
  <Link
    to="/transactions"
    search={searchFor(props)}
    className="view-transactions-arrow"
    aria-label={label}
    title={label}
  >
    <FontAwesomeIcon icon={faArrowRight} />
  </Link>
);

/** A settings page's "12 transactions" text, as a link to those transactions. With none, there's
 * nothing to open, so it stays plain text. */
export const TransactionCountLink = ({
  count,
  children,
  ...props
}: ViewTransactionsProps & { count: number; children: ReactNode }) =>
  count > 0 ? (
    <Link
      to="/transactions"
      search={searchFor(props)}
      className="transaction-count-link"
    >
      {children}
    </Link>
  ) : (
    <>{children}</>
  );
