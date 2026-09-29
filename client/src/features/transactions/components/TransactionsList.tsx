import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronRight } from '@fortawesome/free-solid-svg-icons';
import { useTransactions } from '../hooks/use-transactions';
import { useUpdateTransaction } from '../hooks/use-update-transaction';
import { TransactionsToolbar } from './TransactionsToolbar';
import { CategoryPicker } from './CategoryPicker';
import { EditMultipleTransactionsModal } from './EditMultipleTransactionsModal';
import { formatAmount, formatDateHeading } from '../utils/format';
import { normalizeAmount, sanitizeAmountInput } from '../utils/amount';
import { toIntlLocale } from '../../../i18n/locale';
import {
  amountTooLongToast,
  invalidAmountToast,
  requiredFieldToast,
  updateTransactionFailedToast,
} from '../../../lib/toast';
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
  selectionMode,
  isSelected,
  onToggleSelect,
}: {
  transaction: Transaction;
  language: string;
  locale: string;
  selectionMode: boolean;
  isSelected: boolean;
  onToggleSelect: (id: number) => void;
}) => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isCredit = transaction.category_type !== 'expense';
  const categoryName =
    language === 'fr'
      ? transaction.category_name_fr
      : transaction.category_name_en;

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
      const result = normalizeAmount(trimmed);
      if (!result.valid) {
        if (result.error === 'tooLong') amountTooLongToast();
        else invalidAmountToast();
        return;
      }
      setEditingField(null);
      if (result.value !== transaction.amount) save({ amount: result.value });
      return;
    }

    if (trimmed === '') {
      requiredFieldToast();
      return;
    }
    setEditingField(null);
    if (trimmed !== transaction[field]) {
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

  if (selectionMode) {
    return (
      <li
        className="transactions-row transactions-row--selectable"
        onClick={() => onToggleSelect(transaction.id)}
      >
        <input
          type="checkbox"
          className="transactions-row-checkbox"
          checked={isSelected}
          onChange={() => onToggleSelect(transaction.id)}
          onClick={(event) => event.stopPropagation()}
          aria-label={t('transactions.edit.selectTransaction', {
            merchant: transaction.merchant,
          })}
        />
        <span className="transactions-row-cell transactions-row-merchant">
          {transaction.merchant}
        </span>
        <span className="transactions-row-cell transactions-row-category">
          {categoryName}
        </span>
        <span className="transactions-row-cell transactions-row-account">
          {transaction.account}
        </span>
        <span
          className={
            isCredit
              ? 'transactions-row-amount transactions-row-amount--credit'
              : 'transactions-row-amount'
          }
        >
          {isCredit ? '+' : ''}
          {formatAmount(Number(transaction.amount), locale)}
        </span>
      </li>
    );
  }

  return (
    <li className="transactions-row">
      {editingField === 'merchant' ? (
        <input
          className="transactions-row-cell transactions-row-input"
          aria-label={t('transactions.add.merchant', 'Merchant')}
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
          aria-label={t('transactions.add.account', 'Account')}
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
          aria-label={t('transactions.add.amount', 'Amount')}
          inputMode="decimal"
          value={draft}
          onChange={(event) =>
            setDraft(sanitizeAmountInput(event.target.value))
          }
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

      <button
        type="button"
        className="transactions-row-chevron-button"
        aria-label={t('transactions.edit.openTransaction')}
        onClick={() =>
          navigate({
            to: '/transactions/$transactionId',
            params: { transactionId: String(transaction.id) },
            search: (prev) => prev,
          })
        }
      >
        <FontAwesomeIcon
          icon={faChevronRight}
          className="transactions-row-chevron"
        />
      </button>
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

  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [bulkEditOpen, setBulkEditOpen] = useState(false);

  const allIds = useMemo(
    () => (transactions ?? []).map((transaction) => transaction.id),
    [transactions],
  );

  const cancelSelection = () => {
    setSelectionMode(false);
    setSelectedIds(new Set());
  };

  const toggleSelect = (id: number) => {
    setSelectedIds((previous) => {
      const next = new Set(previous);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    setSelectedIds((previous) =>
      previous.size === allIds.length ? new Set() : new Set(allIds),
    );
  };

  const handleBulkSaved = () => {
    setBulkEditOpen(false);
    cancelSelection();
  };

  // Only active while in selection mode, and not while the bulk-edit panel is open — that panel
  // handles its own Escape (close the panel, keep the selection underneath), same as the
  // single-transaction edit panel.
  useEffect(() => {
    if (!selectionMode || bulkEditOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isTyping = !!target && ['INPUT', 'TEXTAREA'].includes(target.tagName);

      if (event.key === 'Escape') {
        cancelSelection();
      } else if (
        !isTyping &&
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === 'a'
      ) {
        event.preventDefault();
        setSelectedIds(new Set(allIds));
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectionMode, bulkEditOpen, allIds]);

  return (
    <div className="transactions-card">
      <TransactionsToolbar
        selectionMode={selectionMode}
        selectedCount={selectedIds.size}
        totalCount={allIds.length}
        onEnterSelectionMode={() => setSelectionMode(true)}
        onCancelSelection={cancelSelection}
        onToggleSelectAll={toggleSelectAll}
        onOpenBulkEdit={() => setBulkEditOpen(true)}
      />
      <div className="transactions-card-body">
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
                  <span>
                    {formatAmount(dailyTotal(group.transactions), locale)}
                  </span>
                </li>
                {group.transactions.map((transaction) => (
                  <TransactionRow
                    key={transaction.id}
                    transaction={transaction}
                    language={language}
                    locale={locale}
                    selectionMode={selectionMode}
                    isSelected={selectedIds.has(transaction.id)}
                    onToggleSelect={toggleSelect}
                  />
                ))}
              </Fragment>
            ))}
          </ul>
        )}
      </div>
      {bulkEditOpen && (
        <EditMultipleTransactionsModal
          transactionIds={Array.from(selectedIds)}
          onClose={() => setBulkEditOpen(false)}
          onSaved={handleBulkSaved}
        />
      )}
    </div>
  );
};
