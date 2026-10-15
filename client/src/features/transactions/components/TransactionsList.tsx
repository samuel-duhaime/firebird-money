import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronRight } from '@fortawesome/free-solid-svg-icons';
import { useAccounts } from '../../accounts/hooks/use-accounts';
import { accountLabel } from '../../accounts/utils/format';
import { useTransactions } from '../hooks/use-transactions';
import { useUpdateTransaction } from '../hooks/use-update-transaction';
import { useSettings } from '../../settings/hooks/use-settings';
import { useUpdateSettings } from '../../settings/hooks/use-update-settings';
import { TransactionsToolbar } from './TransactionsToolbar';
import { CategoryPicker } from './CategoryPicker';
import { MerchantPicker } from './MerchantPicker';
import { TagPicker } from '../../tags/components/TagPicker';
import { EditMultipleTransactionsModal } from './EditMultipleTransactionsModal';
import { ViewTransactionsArrow } from './ViewTransactionsLink';
import { formatAmount, formatDateHeading } from '../utils/format';
import { normalizeAmount, sanitizeAmountInput } from '../utils/amount';
import { toIntlLocale } from '../../../i18n/locale';
import {
  columnVisibilityFromSettings,
  rowGridTemplateColumns,
  toggleColumnPatch,
} from '../utils/column-visibility';
import type {
  ColumnVisibility,
  OptionalColumn,
} from '../utils/column-visibility';
import {
  amountTooLongToast,
  invalidAmountToast,
  updateSettingsFailedToast,
  updateTransactionFailedToast,
} from '../../../lib/toast';
import type { Transaction } from '../utils/types';
import type { TransactionPatch } from '../utils/api';
import { Select } from '../../../components/Select';
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

/** The row's click-to-edit text field. Category, merchant, and account are pick-from-a-list fields
 * (`CategoryPicker`, `MerchantPicker`, `Select`) that commit on pick instead. */
type EditableField = 'amount';

const TransactionRow = ({
  transaction,
  language,
  locale,
  selectionMode,
  isSelected,
  onToggleSelect,
  columnVisibility,
}: {
  transaction: Transaction;
  language: string;
  locale: string;
  selectionMode: boolean;
  isSelected: boolean;
  onToggleSelect: (id: number) => void;
  columnVisibility: ColumnVisibility;
}) => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const isCredit = transaction.category_type !== 'expense';
  const categoryName =
    language === 'fr'
      ? transaction.category_name_fr
      : transaction.category_name_en;

  const updateTransactionMutation = useUpdateTransaction();
  const { data: accounts } = useAccounts();
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

    if (!editingField) return;
    commitInFlightRef.current = true;
    queueMicrotask(() => {
      commitInFlightRef.current = false;
    });

    const result = normalizeAmount(draft.trim());
    if (!result.valid) {
      if (result.error === 'tooLong') amountTooLongToast();
      else invalidAmountToast();
      return;
    }
    setEditingField(null);
    if (result.value !== transaction.amount) save({ amount: result.value });
  };

  const handleAccountChange = (accountId: number) => {
    if (accountId !== transaction.account_id) save({ account_id: accountId });
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

  const handleMerchantSelect = (merchantId: number) => {
    if (merchantId === transaction.merchant_id) return;
    save({ merchant_id: merchantId });
  };

  const tagIds = transaction.tags.map((tag) => tag.id);
  // TODO: this computes `next` straight from `transaction.tags` (this render's server data), so
  // two rapid toggles can both read the pre-save tag set and the second save can clobber the
  // first. See the matching TODO on EditTransactionModal's handleTagToggle — needs the same
  // shared, synchronously-updated draft rather than each row deriving from `transaction.tags`.
  const handleTagToggle = (tagId: number) => {
    const next = tagIds.includes(tagId)
      ? tagIds.filter((id) => id !== tagId)
      : [...tagIds, tagId];
    save({ tag_ids: next });
  };

  if (selectionMode) {
    return (
      <li
        className="transactions-row transactions-row--selectable"
        style={{
          gridTemplateColumns: rowGridTemplateColumns(columnVisibility, {
            selectable: true,
          }),
        }}
        onClick={() => onToggleSelect(transaction.id)}
      >
        <input
          type="checkbox"
          className="transactions-row-checkbox"
          checked={isSelected}
          onChange={() => onToggleSelect(transaction.id)}
          onClick={(event) => event.stopPropagation()}
          aria-label={t('transactions.edit.selectTransaction', {
            merchant: transaction.merchant_name,
          })}
        />
        <span className="transactions-row-cell transactions-row-merchant">
          {transaction.merchant_name}
        </span>
        {columnVisibility.category && (
          <span className="transactions-row-cell transactions-row-category">
            {categoryName}
          </span>
        )}
        {columnVisibility.tags && (
          <span className="transactions-row-cell transactions-row-tags">
            {transaction.tags.map((tag) => (
              <span
                key={tag.id}
                className="transactions-row-tag-dot"
                style={{ backgroundColor: tag.color }}
                title={tag.name}
              />
            ))}
          </span>
        )}
        {columnVisibility.account && (
          <span className="transactions-row-cell transactions-row-account">
            {transaction.account_name}
          </span>
        )}
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
    <li
      className="transactions-row"
      style={{
        gridTemplateColumns: rowGridTemplateColumns(columnVisibility, {
          selectable: false,
        }),
      }}
    >
      <div className="transactions-row-cell-group">
        <MerchantPicker
          merchantId={transaction.merchant_id}
          label={transaction.merchant_name}
          className="transactions-row-cell transactions-row-merchant transactions-row-cell--editable"
          onSelect={handleMerchantSelect}
          ariaLabel={t('transactions.add.merchant', 'Merchant')}
        />
        <ViewTransactionsArrow
          filter="merchant_ids"
          id={transaction.merchant_id}
          label={t('transactions.view.merchant')}
        />
      </div>

      {columnVisibility.category && (
        <div className="transactions-row-cell-group">
          <CategoryPicker
            categoryId={transaction.category_id}
            label={categoryName}
            className="transactions-row-cell transactions-row-category transactions-row-cell--editable"
            onSelect={handleCategorySelect}
          />
          <ViewTransactionsArrow
            filter="category_ids"
            id={transaction.category_id}
            label={t('transactions.view.category')}
          />
        </div>
      )}

      {columnVisibility.tags && (
        <TagPicker
          selectedTagIds={tagIds}
          onToggle={handleTagToggle}
          triggerClassName="transactions-row-cell transactions-row-tags transactions-row-cell--editable"
          ariaLabel={t('transactions.add.tags')}
        />
      )}

      {columnVisibility.account && (
        <div className="transactions-row-cell-group">
          <Select
            variant="cell"
            className="transactions-row-cell transactions-row-account"
            aria-label={t('transactions.add.account', 'Account')}
            value={transaction.account_id}
            onChange={(event) =>
              handleAccountChange(Number(event.target.value))
            }
          >
            {/* Until the accounts list loads, still show the current one. */}
            {!accounts && (
              <option value={transaction.account_id}>
                {transaction.account_name}
              </option>
            )}
            {accounts?.map((account) => (
              <option key={account.id} value={account.id}>
                {accountLabel(account)}
              </option>
            ))}
          </Select>
          <ViewTransactionsArrow
            filter="account_ids"
            id={transaction.account_id}
            label={t('transactions.view.account')}
          />
        </div>
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
  const query = routeApi.useSearch();
  const { data: transactions, isPending, isError } = useTransactions(query);

  const { data: settings } = useSettings();
  const updateSettingsMutation = useUpdateSettings();
  const columnVisibility = columnVisibilityFromSettings(settings);
  const handleToggleColumn = (column: OptionalColumn) => {
    updateSettingsMutation.mutate(toggleColumnPatch(column, columnVisibility), {
      onError: updateSettingsFailedToast,
    });
  };

  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [bulkEditOpen, setBulkEditOpen] = useState(false);

  const allIds = useMemo(
    () => (transactions ?? []).map((transaction) => transaction.id),
    [transactions],
  );

  // The top menu's search/date-range controls stay reachable while selection mode is active, so
  // the loaded transactions (and `allIds`) can change out from under an in-progress selection —
  // drop any selected id that's no longer in the current list, so the toolbar count, the
  // "select all" checkbox, and the bulk-edit panel never act on rows the user can't see anymore.
  // Skipped while `transactions` is `undefined` (a new filter's query key is still loading,
  // without `keepPreviousData`) — otherwise that transient empty state would wipe the whole
  // selection right before the real, still-relevant results arrive.
  useEffect(() => {
    if (!transactions) return;
    setSelectedIds((previous) => {
      const next = new Set([...previous].filter((id) => allIds.includes(id)));
      return next.size === previous.size ? previous : next;
    });
  }, [transactions, allIds]);

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
      const isTyping =
        !!target && ['INPUT', 'TEXTAREA'].includes(target.tagName);

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
        columnVisibility={columnVisibility}
        onEnterSelectionMode={() => setSelectionMode(true)}
        onCancelSelection={cancelSelection}
        onToggleSelectAll={toggleSelectAll}
        onOpenBulkEdit={() => setBulkEditOpen(true)}
        onToggleColumn={handleToggleColumn}
      />
      <div className="transactions-card-body">
        {isPending && (
          <p className="transactions-status">
            {t('transactions.list.loading')}
          </p>
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
                    columnVisibility={columnVisibility}
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
