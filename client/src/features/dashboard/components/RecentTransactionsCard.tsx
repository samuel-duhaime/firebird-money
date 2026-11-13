import { useState } from 'react';
import type { ReactNode } from 'react';
import { Link, useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { useTransactions } from '../../transactions/hooks/use-transactions';
import { useReviewCounts } from '../../transactions/hooks/use-review-counts';
import { TransactionRow } from '../../transactions/components/TransactionsList';
import {
  ReviewEmptyState,
  ReviewViewButton,
} from '../../transactions/components/ReviewViewButton';
import { EditTransactionModal } from '../../transactions/components/EditTransactionModal';
import { toNeedsReview } from '../../transactions/utils/filters';
import type { ReviewView } from '../../transactions/utils/filters';
import type { ColumnVisibility } from '../../transactions/utils/column-visibility';
import { toIntlLocale } from '../../../i18n/locale';
import '../../transactions/components/TransactionsList.css';
import '../../transactions/components/TransactionsToolbar.css';
import './RecentTransactionsCard.css';

/** How many transactions the card lists. */
export const RECENT_TRANSACTIONS_COUNT = 5;

/** The card has room for merchant, category and amount only, whatever the Transactions page
 * shows. */
const COLUMNS: ColumnVisibility = {
  category: true,
  tags: false,
  account: false,
  owner: false,
};

/** Merchant and category share the card's width evenly, so neither gets squeezed in half the
 * dashboard; the review button's track comes first in the review views. */
const gridTemplateColumns = (showReviewButton: boolean): string =>
  [
    ...(showReviewButton ? ['20px'] : []),
    'minmax(0, 1fr)',
    'minmax(0, 1fr)',
    '96px',
    '16px',
  ].join(' ');

const noop = () => {};

/** The dashboard's most recent transactions, edited with the Transactions page's own row. */
export const RecentTransactionsCard = ({
  moveHandle,
}: {
  /** The dashboard's grip for moving this card, in its header's left padding. */
  moveHandle?: ReactNode;
}) => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const locale = toIntlLocale(language);
  const [reviewView, setReviewView] = useState<ReviewView>('all');
  const [openId, setOpenId] = useState<number | null>(null);
  const navigate = useNavigate();

  const needsReview = toNeedsReview(reviewView);
  const {
    data: transactions,
    isPending,
    isError,
  } = useTransactions({ needs_review: needsReview });
  const {
    counts: reviewCounts,
    isPending: isCountsPending,
    isError: isCountsError,
  } = useReviewCounts();
  const recent = transactions?.slice(0, RECENT_TRANSACTIONS_COUNT);

  return (
    <section
      className="transactions-card recent-transactions-card"
      aria-labelledby="recent-transactions-title"
    >
      <header className="transactions-toolbar recent-transactions-header">
        {moveHandle}
        <div className="recent-transactions-heading">
          <h2
            id="recent-transactions-title"
            className="recent-transactions-title"
          >
            <Link
              to="/transactions"
              search={{ needs_review: needsReview }}
              className="recent-transactions-link"
            >
              {t('dashboard.transactions.title')}
            </Link>
          </h2>
          <span className="recent-transactions-subtitle">
            {t('dashboard.transactions.subtitle')}
          </span>
        </div>
        <ReviewViewButton
          view={reviewView}
          counts={reviewCounts}
          onChange={setReviewView}
        />
      </header>
      <div className="transactions-card-body">
        {isPending && (
          <p className="transactions-status">
            {t('transactions.list.loading')}
          </p>
        )}
        {isError && (
          <p className="transactions-status">{t('transactions.list.error')}</p>
        )}
        {recent &&
          recent.length === 0 &&
          (reviewView === 'all' || reviewCounts[reviewView] > 0 ? (
            <p className="transactions-status">
              {t('transactions.list.empty')}
            </p>
          ) : isCountsPending ? (
            <p className="transactions-status">
              {t('transactions.list.loading')}
            </p>
          ) : isCountsError ? (
            <p className="transactions-status">
              {t('transactions.list.error')}
            </p>
          ) : (
            <ReviewEmptyState
              onViewAll={() => navigate({ to: '/transactions' })}
            />
          ))}
        {recent && recent.length > 0 && (
          <ul className="transactions-rows">
            {recent.map((transaction) => (
              <TransactionRow
                key={transaction.id}
                transaction={transaction}
                language={language}
                locale={locale}
                selectionMode={false}
                isSelected={false}
                onToggleSelect={noop}
                columnVisibility={COLUMNS}
                showReviewButton={reviewView !== 'all' && !transaction.reviewed}
                gridTemplateColumns={gridTemplateColumns(
                  reviewView !== 'all' && !transaction.reviewed,
                )}
                onOpen={setOpenId}
              />
            ))}
          </ul>
        )}
      </div>
      {openId !== null && (
        <EditTransactionModal
          transactionId={openId}
          onClose={() => setOpenId(null)}
        />
      )}
    </section>
  );
};
