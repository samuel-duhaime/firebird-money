import { Fragment, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronRight } from '@fortawesome/free-solid-svg-icons';
import { useTransactions } from '../hooks/use-transactions';
import { useUpdateTransaction } from '../hooks/use-update-transaction';
import { TransactionsToolbar } from './TransactionsToolbar';
import { CategoryPicker } from './CategoryPicker';
import { formatAmount, formatDateHeading } from '../utils/format';
import { normalizeAmount, sanitizeAmountInput } from '../utils/amount';
import { toIntlLocale } from '../../../i18n/locale';
import { invalidAmountToast, updateTransactionFailedToast } from '../../../lib/toast';
import type { Transaction } from '../utils/types';
import type { TransactionPatch } from '../utils/api';
import './TransactionsList.css';

/** Groups transactions by date, assuming they already arrive sorted with same-date rows adjacent. */
const groupByDate = (
  transactions: Transaction[],
): { date: string; transactions: Transaction[] }[] => {
  const groups: { date: string; transactions: Transaction[] }[] = [];
  for (const transaction of transactions) {
    const currentGroup = groups.at(-1);
    if (currentGroup?.date === transaction.date) {
      currentGroup.transactions.push(transaction);
    } else {
      groups.push({ date: transaction.date, transactions: [transaction] });
    }
  }
  return groups;
};

/** A day's total is its net spend: credits (income/transfers) don't offset expenses. */
const dailyTotal = (transactions: Transaction[]): number =>
  transactions
    .filter((transaction) => transaction.category_type === 'expense')
    .reduce((sum, transaction) => sum + Number(transaction.amount), 0);

/** The row's directly-editable text fields. Category is
 * edited through `CategoryPicker` instead, since it's a pick-from-a-list field, not free text. */
type EditableField = 'merchant' | 'account' | 'amount';

const TransactionRow = ({
  transaction,
  language,
  locale,
}: {
  transaction: Transaction;
  language: string;
  locale: string;
}) => {
  const isCredit = transaction.category_type !== 'expense';
  const categoryName =
    language === 'fr' ? transaction.category_name_fr : transaction.category_name_en;

  const updateTransactionMutation = useUpdateTransaction();
  const [editingField, setEditingField] = useState<EditableField | null>(null);
  const [draft, setDraft] = useState('');
  // Guards against the field's input firing a second, native blur-on-unmount once editing has
  // already been closed by an explicit Enter/Escape in the same tick (see handleKeyDown).
  const commitInFlightRef = useRef(false);
  const cancelledRef = useRef(false);

  const startEdit = (field: EditableField, value: string) => {
    setDraft(value);
    setEditingField(field);
  };

  const save = (patch: TransactionPatch) => {
    updateTransactionMutation.mutate(
      { id: transaction.id, patch },
      { onError: () => updateTransactionFailedToast() },
    );
  };

  const commit = () => {
    if (commitInFlightRef.current) return;
    if (cancelledRef.current) {
      cancelledRef.current = false;
      return;
    }

    const field = editingField;
    if (!field) return;
    commitInFlightRef.current = true;
    queueMicrotask(() => {
      commitInFlightRef.current = false;
    });

    const trimmed = draft.trim();

    if (field === 'amount') {
      const normalized = normalizeAmount(trimmed);
      if (normalized === null) {
        invalidAmountToast();
        return;
      }
      setEditingField(null);
      if (normalized !== transaction.amount) save({ amount: normalized });
      return;
    }

    setEditingField(null);
    if (trimmed !== '' && trimmed !== transaction[field]) {
      save({ [field]: trimmed });
    }
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Escape') {
      cancelledRef.current = true;
      setEditingField(null);
    }
  };

  const handleCategorySelect = (categoryId: number) => {
    if (categoryId === transaction.category_id) return;
    save({ category_id: categoryId });
  };

  return (
    <li className="transactions-row">
      {editingField === 'merchant' ? (
        <input
          className="transactions-row-cell transactions-row-input"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={commit}
          autoFocus
        />
      ) : (
        <button
          type="button"
          className="transactions-row-cell transactions-row-merchant transactions-row-cell--editable"
          onClick={() => startEdit('merchant', transaction.merchant)}
        >
          {transaction.merchant}
        </button>
      )}

      <CategoryPicker
        categoryId={transaction.category_id}
        label={categoryName}
        className="transactions-row-cell transactions-row-category transactions-row-cell--editable"
        onSelect={handleCategorySelect}
      />

      {editingField === 'account' ? (
        <input
          className="transactions-row-cell transactions-row-input"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={commit}
          autoFocus
        />
      ) : (
        <button
          type="button"
          className="transactions-row-cell transactions-row-account transactions-row-cell--editable"
          onClick={() => startEdit('account', transaction.account)}
        >
          {transaction.account}
        </button>
      )}

      {editingField === 'amount' ? (
        <input
          className="transactions-row-input transactions-row-input--amount"
          inputMode="decimal"
          value={draft}
          onChange={(event) => setDraft(sanitizeAmountInput(event.target.value))}
          onKeyDown={handleKeyDown}
          onBlur={commit}
          autoFocus
        />
      ) : (
        <button
          type="button"
          className={
            isCredit
              ? 'transactions-row-amount transactions-row-amount--credit transactions-row-cell--editable'
              : 'transactions-row-amount transactions-row-cell--editable'
          }
          onClick={() => startEdit('amount', transaction.amount)}
        >
          {isCredit ? '+' : ''}
          {formatAmount(Number(transaction.amount), locale)}
        </button>
      )}

      <FontAwesomeIcon
        icon={faChevronRight}
        className="transactions-row-chevron"
      />
    </li>
  );
};

const routeApi = getRouteApi('/_app/transactions');

export const TransactionsList = () => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const locale = toIntlLocale(language);
  const { search, order, start_date, end_date } = routeApi.useSearch();
  const {
    data: transactions,
    isPending,
    isError,
  } = useTransactions(search, order, start_date, end_date);

  return (
    <div className="transactions-card">
      <TransactionsToolbar />
      {isPending && (
        <p className="transactions-status">{t('transactions.list.loading')}</p>
      )}
      {isError && (
        <p className="transactions-status">{t('transactions.list.error')}</p>
      )}
      {transactions && transactions.length === 0 && (
        <p className="transactions-status">{t('transactions.list.empty')}</p>
      )}
      {transactions && transactions.length > 0 && (
        <ul className="transactions-rows">
          {groupByDate(transactions).map((group, index) => (
            <Fragment key={`${group.date}-${index}`}>
              <li className="transactions-date-header">
                <span>{formatDateHeading(group.date, locale)}</span>
                <span>{formatAmount(dailyTotal(group.transactions), locale)}</span>
              </li>
              {group.transactions.map((transaction) => (
                <TransactionRow
                  key={transaction.id}
                  transaction={transaction}
                  language={language}
                  locale={locale}
                />
              ))}
            </Fragment>
          ))}
        </ul>
      )}
    </div>
  );
};
