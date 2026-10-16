import { useId, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faClone,
  faTriangleExclamation,
} from '@fortawesome/free-solid-svg-icons';
import { Select } from '../../../../components/Select';
import { useAccounts } from '../../../accounts/hooks/use-accounts';
import { accountLabel } from '../../../accounts/utils/format';
import { useCategories } from '../../../categories/hooks/use-categories';
import { useCategoryGroups } from '../../../categories/hooks/use-category-groups';
import { useMerchants } from '../../../merchants/hooks/use-merchants';
import { TagPicker } from '../../../tags/components/TagPicker';
import { toIntlLocale } from '../../../../i18n/locale';
import { CategoryPicker } from '../CategoryPicker';
import { MerchantPicker } from '../MerchantPicker';
import { formatAmount } from '../../utils/format';
import type {
  StatementTransaction,
  SuggestedAccount,
} from '../../utils/import';

/** A statement line under review: what the server read, plus whether the user keeps it and, when
 * the statement named an account the household doesn't have, whether to create it for this line. */
export type ReviewLine = StatementTransaction & {
  key: number;
  selected: boolean;
  useNewAccount: boolean;
};

/** "New: RBC Mastercard (…9999)". */
const newAccountLabel = (account: SuggestedAccount, prefix: string) =>
  `${prefix} ${account.institution_name} ${accountLabel(account)}`;

/** The account `<select>`'s value for a line: an account id, `new`, or empty (none yet). */
const accountChoice = (line: ReviewLine) =>
  line.account_id !== null
    ? String(line.account_id)
    : line.useNewAccount
      ? 'new'
      : '';

type Filter = 'all' | 'look' | 'newMerchants' | 'duplicates';

/** A line can't be added until it has both a category and an account. */
const isMissingInfo = (line: ReviewLine) =>
  line.category_id === null ||
  (line.account_id === null && !line.useNewAccount);

const FILTERS: Record<Filter, (line: ReviewLine) => boolean> = {
  all: () => true,
  look: (line) =>
    !line.category_confident ||
    line.new_merchant_name !== null ||
    line.duplicate_of_transaction_id !== null ||
    isMissingInfo(line),
  newMerchants: (line) => line.new_merchant_name !== null,
  duplicates: (line) => line.duplicate_of_transaction_id !== null,
};

/** "Sep 2" / "2 sept." — the statement's own dates, without UTC-shifting the day. */
const formatShortDate = (isoDate: string, locale: string) => {
  const [year, month, day] = isoDate.split('-').map(Number);
  return new Date(year, month - 1, day).toLocaleDateString(locale, {
    month: 'short',
    day: 'numeric',
  });
};

type StatementReviewProps = {
  fileName: string;
  lines: ReviewLine[];
  onLinesChange: (lines: ReviewLine[]) => void;
  onDiscard: () => void;
  onAdd: () => void;
  isAdding: boolean;
};

/** The last step: check, fix, and pick which lines to add. */
export const StatementReview = ({
  fileName,
  lines,
  onLinesChange,
  onDiscard,
  onAdd,
  isAdding,
}: StatementReviewProps) => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const locale = toIntlLocale(language);
  const { data: accounts } = useAccounts();
  const { data: categories } = useCategories();
  const { data: categoryGroups } = useCategoryGroups();
  const { data: merchants } = useMerchants();
  const [filter, setFilter] = useState<Filter>('all');
  const accountForAllId = useId();

  const categoryById = useMemo(
    () =>
      new Map((categories ?? []).map((category) => [category.id, category])),
    [categories],
  );
  const groupTypeById = useMemo(
    () =>
      new Map((categoryGroups ?? []).map((group) => [group.id, group.type])),
    [categoryGroups],
  );
  const merchantNameById = useMemo(
    () =>
      new Map(
        (merchants ?? []).map((merchant) => [merchant.id, merchant.name]),
      ),
    [merchants],
  );

  const categoryName = (categoryId: number | null) => {
    const category =
      categoryId === null ? undefined : categoryById.get(categoryId);
    if (!category) return t('transactions.import.review.chooseCategory');
    return language === 'fr' ? category.name_fr : category.name_en;
  };
  const isCredit = (line: ReviewLine) => {
    const category =
      line.category_id === null
        ? undefined
        : categoryById.get(line.category_id);
    return category
      ? groupTypeById.get(category.group_id) !== 'expense'
      : false;
  };

  const update = (key: number, patch: Partial<ReviewLine>) =>
    onLinesChange(
      lines.map((line) => (line.key === key ? { ...line, ...patch } : line)),
    );

  const shown = lines.filter(FILTERS[filter]);
  const allShownSelected =
    shown.length > 0 && shown.every((line) => line.selected);
  const toggleAllShown = () => {
    const shownKeys = new Set(shown.map((line) => line.key));
    onLinesChange(
      lines.map((line) =>
        shownKeys.has(line.key)
          ? { ...line, selected: !allShownSelected }
          : line,
      ),
    );
  };

  // A statement is for one account, so its suggestion is the same on every line that has one.
  const suggestedAccount =
    lines.find((line) => line.new_account !== null)?.new_account ?? null;
  const chooseAccount = (line: ReviewLine, choice: string): ReviewLine =>
    choice === 'new'
      ? {
          ...line,
          account_id: null,
          new_account: line.new_account ?? suggestedAccount,
          useNewAccount: true,
        }
      : { ...line, account_id: Number(choice), useNewAccount: false };
  const newAccountOption = suggestedAccount && (
    <option value="new">
      {newAccountLabel(
        suggestedAccount,
        t('transactions.import.review.newAccountOption'),
      )}
    </option>
  );

  const selected = lines.filter((line) => line.selected);
  const missingCount = selected.filter(isMissingInfo).length;
  const newMerchantCount = new Set(
    selected
      .map((line) => line.new_merchant_name?.trim().toLowerCase())
      .filter((name): name is string => !!name),
  ).size;
  const newAccountCount = new Set(
    selected
      .filter((line) => accountChoice(line) === 'new' && line.new_account)
      .map((line) => JSON.stringify(line.new_account)),
  ).size;
  const sum = (predicate: (line: ReviewLine) => boolean) =>
    selected
      .filter((line) => line.category_id !== null && predicate(line))
      .reduce((total, line) => total + Number(line.amount), 0);
  const totalIn = sum(isCredit);
  const totalOut = sum((line) => !isCredit(line));

  if (lines.length === 0) {
    return (
      <div className="statement-import">
        <p className="statement-import-intro">
          {t('transactions.import.review.empty')}
        </p>
        <div>
          <button
            type="button"
            className="statement-button"
            onClick={onDiscard}
          >
            {t('transactions.import.review.tryAnother')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="statement-review">
      <div className="statement-review-header">
        <div>
          <h2 className="statement-review-title">
            {t('transactions.import.review.title', { count: lines.length })}
          </h2>
          <p className="statement-review-subtitle">
            {t('transactions.import.review.subtitle', { file: fileName })}
          </p>
        </div>
        <div className="statement-review-account">
          <label htmlFor={accountForAllId}>
            {t('transactions.import.review.accountForAll')}
          </label>
          <Select
            id={accountForAllId}
            variant="compact"
            value=""
            isPlaceholder
            onChange={(event) => {
              const choice = event.target.value;
              onLinesChange(lines.map((line) => chooseAccount(line, choice)));
            }}
          >
            <option value="" disabled hidden>
              {t('transactions.import.review.chooseAccount')}
            </option>
            {newAccountOption}
            {accounts?.map((account) => (
              <option key={account.id} value={account.id}>
                {accountLabel(account)}
              </option>
            ))}
          </Select>
        </div>
      </div>

      <div
        className="statement-filters"
        role="group"
        aria-label={t('transactions.import.review.filtersLabel')}
      >
        {(Object.keys(FILTERS) as Filter[]).map((name) => (
          <button
            key={name}
            type="button"
            aria-pressed={filter === name}
            className={
              filter === name
                ? 'statement-filter statement-filter--active'
                : 'statement-filter'
            }
            onClick={() => setFilter(name)}
          >
            {t(`transactions.import.review.filters.${name}`)}
            <span className="statement-filter-count">
              {lines.filter(FILTERS[name]).length}
            </span>
          </button>
        ))}
      </div>

      <div className="statement-table-wrapper">
        <table className="statement-table">
          <thead>
            <tr>
              <th scope="col" className="statement-table-check">
                <input
                  type="checkbox"
                  className="transactions-row-checkbox"
                  aria-label={t('transactions.import.review.selectAll')}
                  checked={allShownSelected}
                  onChange={toggleAllShown}
                />
              </th>
              <th scope="col">
                {t('transactions.import.review.columns.date')}
              </th>
              <th scope="col">
                {t('transactions.import.review.columns.merchant')}
              </th>
              <th scope="col">
                {t('transactions.import.review.columns.category')}
              </th>
              <th scope="col">
                {t('transactions.import.review.columns.account')}
              </th>
              <th scope="col">
                {t('transactions.import.review.columns.tags')}
              </th>
              <th scope="col" className="statement-table-amount">
                {t('transactions.import.review.columns.amount')}
              </th>
            </tr>
          </thead>
          <tbody>
            {shown.map((line) => {
              const merchantName =
                line.merchant_id !== null
                  ? (merchantNameById.get(line.merchant_id) ?? '')
                  : (line.new_merchant_name ?? '');
              const date = formatShortDate(line.date, locale);
              const rowClass = [
                'statement-row',
                line.selected ? '' : 'statement-row--skipped',
                line.duplicate_of_transaction_id !== null
                  ? 'statement-row--duplicate'
                  : '',
              ]
                .filter(Boolean)
                .join(' ');
              return (
                <tr key={line.key} className={rowClass}>
                  <td className="statement-table-check">
                    <input
                      type="checkbox"
                      className="transactions-row-checkbox"
                      aria-label={t('transactions.import.review.selectLine', {
                        merchant: merchantName,
                        date,
                      })}
                      checked={line.selected}
                      onChange={() =>
                        update(line.key, { selected: !line.selected })
                      }
                    />
                  </td>
                  <td className="statement-table-date">{date}</td>
                  <td>
                    <div className="statement-merchant">
                      <span className="statement-merchant-line">
                        <MerchantPicker
                          merchantId={line.merchant_id}
                          label={merchantName}
                          className="statement-picker statement-picker--strong"
                          ariaLabel={`${t('transactions.import.review.columns.merchant')}: ${merchantName}`}
                          onSelect={(merchantId) =>
                            update(line.key, {
                              merchant_id: merchantId,
                              new_merchant_name: null,
                            })
                          }
                        />
                        {line.new_merchant_name !== null && (
                          <span className="statement-badge statement-badge--new">
                            {t('transactions.import.review.newMerchant')}
                          </span>
                        )}
                      </span>
                      <span className="statement-merchant-raw">
                        {line.original_statement}
                      </span>
                      {line.duplicate_of_transaction_id !== null && (
                        <span className="statement-note">
                          <FontAwesomeIcon icon={faClone} aria-hidden="true" />
                          {t('transactions.import.review.duplicate')}
                        </span>
                      )}
                    </div>
                  </td>
                  <td>
                    <CategoryPicker
                      categoryId={line.category_id}
                      label={categoryName(line.category_id)}
                      className={
                        line.category_id === null || !line.category_confident
                          ? 'statement-picker statement-picker--box statement-picker--warning'
                          : 'statement-picker statement-picker--box'
                      }
                      ariaLabel={`${t('transactions.import.review.columns.category')}: ${categoryName(line.category_id)}`}
                      onSelect={(categoryId) =>
                        update(line.key, {
                          category_id: categoryId,
                          category_confident: true,
                        })
                      }
                    />
                    {line.category_id !== null && !line.category_confident && (
                      <span className="statement-note">
                        <FontAwesomeIcon
                          icon={faTriangleExclamation}
                          aria-hidden="true"
                        />
                        {t('transactions.import.review.unsure')}
                      </span>
                    )}
                  </td>
                  <td>
                    <Select
                      variant="field"
                      className={
                        accountChoice(line) === ''
                          ? 'statement-select--warning'
                          : ''
                      }
                      aria-label={t(
                        'transactions.import.review.columns.account',
                      )}
                      value={accountChoice(line)}
                      isPlaceholder={accountChoice(line) === ''}
                      onChange={(event) =>
                        onLinesChange(
                          lines.map((other) =>
                            other.key === line.key
                              ? chooseAccount(other, event.target.value)
                              : other,
                          ),
                        )
                      }
                    >
                      <option value="" disabled hidden>
                        {t('transactions.import.review.chooseAccount')}
                      </option>
                      {newAccountOption}
                      {accounts?.map((account) => (
                        <option key={account.id} value={account.id}>
                          {accountLabel(account)}
                        </option>
                      ))}
                    </Select>
                    {accountChoice(line) === 'new' && (
                      <span className="statement-badge statement-badge--new statement-badge--below">
                        {t('transactions.import.review.newAccount')}
                      </span>
                    )}
                  </td>
                  <td>
                    <TagPicker
                      selectedTagIds={line.tag_ids}
                      triggerClassName="statement-picker"
                      ariaLabel={t('transactions.import.review.columns.tags')}
                      onToggle={(tagId) =>
                        update(line.key, {
                          tag_ids: line.tag_ids.includes(tagId)
                            ? line.tag_ids.filter((id) => id !== tagId)
                            : [...line.tag_ids, tagId],
                        })
                      }
                    />
                  </td>
                  <td
                    className={
                      isCredit(line)
                        ? 'statement-table-amount transactions-row-amount--credit'
                        : 'statement-table-amount'
                    }
                  >
                    {formatAmount(Number(line.amount), locale)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="statement-footer">
        <div className="statement-footer-summary">
          <span>
            {t('transactions.import.review.selected', {
              selected: selected.length,
              total: lines.length,
            })}
          </span>
          <span>
            {t('transactions.import.review.in')}{' '}
            <strong className="statement-total--in">
              {formatAmount(totalIn, locale)}
            </strong>
          </span>
          <span>
            {t('transactions.import.review.out')}{' '}
            <strong className="statement-total--out">
              {formatAmount(totalOut, locale)}
            </strong>
          </span>
          {newMerchantCount > 0 && (
            <span className="statement-footer-muted">
              {t('transactions.import.review.newMerchants', {
                count: newMerchantCount,
              })}
            </span>
          )}
          {newAccountCount > 0 && (
            <span className="statement-footer-muted">
              {t('transactions.import.review.newAccounts', {
                count: newAccountCount,
              })}
            </span>
          )}
          {missingCount > 0 && (
            <span className="statement-footer-warning" role="status">
              <FontAwesomeIcon
                icon={faTriangleExclamation}
                aria-hidden="true"
              />
              {t('transactions.import.review.missing', { count: missingCount })}
            </span>
          )}
        </div>
        <div className="statement-footer-actions">
          <button
            type="button"
            className="statement-button statement-button--secondary"
            onClick={onDiscard}
            disabled={isAdding}
          >
            {t('transactions.import.review.discard')}
          </button>
          <button
            type="button"
            className="statement-button"
            onClick={onAdd}
            disabled={isAdding || selected.length === 0 || missingCount > 0}
          >
            {isAdding
              ? t('transactions.import.review.adding')
              : t('transactions.import.review.add', { count: selected.length })}
          </button>
        </div>
      </div>
    </div>
  );
};
