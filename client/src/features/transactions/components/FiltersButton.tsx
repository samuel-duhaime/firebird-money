import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faCircleXmark,
  faFilter,
  faMagnifyingGlass,
  faReceipt,
} from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../../components/TopMenuButton';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import { useCategories } from '../../categories/hooks/use-categories';
import { useCategoryGroups } from '../../categories/hooks/use-category-groups';
import { useMerchants } from '../../merchants/hooks/use-merchants';
import { useAccounts } from '../../accounts/hooks/use-accounts';
import { useTags } from '../../tags/hooks/use-tags';
import { accountLabel } from '../../accounts/utils/format';
import { ACCOUNT_TYPES } from '../../accounts/utils/types';
import { toIntlLocale } from '../../../i18n/locale';
import { formatAmount } from '../utils/format';
import { normalizeAmount, sanitizeAmountInput } from '../utils/amount';
import { AMOUNT_OPS, EMPTY_FILTERS, countFilters } from '../utils/filters';
import type { AmountOp, TransactionFilters } from '../utils/filters';
import '../../../components/Popover.css';
import './FiltersButton.css';

const routeApi = getRouteApi('/_app/transactions');

type Section = 'categories' | 'merchants' | 'accounts' | 'tags' | 'amount';

const SECTIONS: Section[] = [
  'categories',
  'merchants',
  'accounts',
  'tags',
  'amount',
];

type IdListKey = 'category_ids' | 'merchant_ids' | 'account_ids' | 'tag_ids';

/** The popover's in-progress edits, only written to the URL on Apply. Amounts stay as typed text
 * until then, so a half-typed value (e.g. "12,") doesn't get coerced away mid-edit. */
type Draft = {
  category_ids: number[];
  merchant_ids: number[];
  account_ids: number[];
  tag_ids: number[];
  untagged: boolean;
  amount_op: AmountOp;
  amount: string;
  amount_max: string;
};

const toDraft = (filters: TransactionFilters): Draft => ({
  category_ids: filters.category_ids ?? [],
  merchant_ids: filters.merchant_ids ?? [],
  account_ids: filters.account_ids ?? [],
  tag_ids: filters.tag_ids ?? [],
  untagged: filters.untagged ?? false,
  amount_op: filters.amount_op ?? 'greater_than',
  amount: filters.amount !== undefined ? String(filters.amount) : '',
  amount_max:
    filters.amount_max !== undefined ? String(filters.amount_max) : '',
});

const parseAmountText = (text: string): number | null => {
  const result = normalizeAmount(text);
  return result.valid ? Number(result.value) : null;
};

/** Whether each amount field holds something Apply can use — empty `amount` means "no amount
 * filter", but `between` needs both ends once either is filled in, in order. */
const amountValidity = (draft: Draft) => {
  const amount = draft.amount === '' ? null : parseAmountText(draft.amount);
  const amountMax =
    draft.amount_max === '' ? null : parseAmountText(draft.amount_max);
  const isAmountValid = draft.amount === '' || amount !== null;
  const isAmountMaxValid =
    draft.amount_op !== 'between' ||
    (draft.amount === '' && draft.amount_max === '') ||
    (amountMax !== null && (amount === null || amountMax >= amount));
  return { amount, amountMax, isAmountValid, isAmountMaxValid };
};

/** `draft` as URL search params, or `null` while an amount field is invalid. */
const toFilters = (draft: Draft): TransactionFilters | null => {
  const { amount, amountMax, isAmountValid, isAmountMaxValid } =
    amountValidity(draft);
  if (!isAmountValid || !isAmountMaxValid) return null;

  const ids = (list: number[]) => (list.length > 0 ? list : undefined);
  const hasAmount = amount !== null;
  return {
    category_ids: ids(draft.category_ids),
    merchant_ids: ids(draft.merchant_ids),
    account_ids: ids(draft.account_ids),
    tag_ids: ids(draft.tag_ids),
    untagged: draft.untagged || undefined,
    amount_op: hasAmount ? draft.amount_op : undefined,
    amount: hasAmount ? amount : undefined,
    amount_max:
      hasAmount && draft.amount_op === 'between'
        ? (amountMax ?? undefined)
        : undefined,
  };
};

/** Adds `ids` to `list` when `checked`, otherwise removes them — keeping `list`'s order. */
const setIds = (list: number[], ids: number[], checked: boolean): number[] =>
  checked
    ? [...list, ...ids.filter((id) => !list.includes(id))]
    : list.filter((id) => !ids.includes(id));

/** A labelled checkbox that can also show the "some but not all" (indeterminate) state, which
 * has no HTML attribute and has to be set on the element itself. */
const CheckboxRow = ({
  checked,
  indeterminate = false,
  onChange,
  variant = 'item',
  children,
}: {
  checked: boolean;
  indeterminate?: boolean;
  onChange: (checked: boolean) => void;
  variant?: 'item' | 'group';
  children: ReactNode;
}) => {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (inputRef.current) inputRef.current.indeterminate = indeterminate;
  }, [indeterminate]);

  return (
    <label className={`filters-popover-row filters-popover-row--${variant}`}>
      <input
        ref={inputRef}
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      {children}
    </label>
  );
};

/** "Select all" / group checkbox state over `ids` against the draft's `selected` list. */
const groupState = (ids: number[], selected: number[]) => {
  const count = ids.filter((id) => selected.includes(id)).length;
  return {
    checked: ids.length > 0 && count === ids.length,
    indeterminate: count > 0 && count < ids.length,
  };
};

/** First-letter badge, the same stand-in for a logo the settings pages use. */
const LetterAvatar = ({ name }: { name: string }) => (
  <span className="filters-popover-avatar" aria-hidden="true">
    {name.charAt(0).toUpperCase()}
  </span>
);

const MerchantAvatar = ({
  name,
  logoUrl,
}: {
  name: string;
  logoUrl: string | null;
}) =>
  logoUrl ? (
    <img className="filters-popover-avatar" src={logoUrl} alt="" />
  ) : (
    <LetterAvatar name={name} />
  );

const TagDot = ({ color }: { color: string }) => (
  <span
    className="filters-popover-tag-dot"
    style={{ background: color }}
    aria-hidden="true"
  />
);

/** One selected filter in the summary column, removable on its own. */
type SummaryItem = {
  key: string;
  icon?: ReactNode;
  label: string;
  onRemove: () => void;
};

export const FiltersButton = () => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const locale = toIntlLocale(language);
  const searchParams = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const searchInputRef = useRef<HTMLInputElement>(null);
  const wasOpenRef = useRef(false);

  const { data: categories } = useCategories();
  const { data: categoryGroups } = useCategoryGroups();
  const { data: merchants } = useMerchants();
  const { data: accounts } = useAccounts();
  const { data: tags } = useTags();

  const [section, setSection] = useState<Section>('categories');
  const [search, setSearch] = useState('');
  const [draft, setDraft] = useState<Draft>(() => toDraft(searchParams));

  const activeCount = countFilters(searchParams);

  // Return focus to the trigger once the popover actually closes, but not on first mount.
  useEffect(() => {
    if (isOpen) {
      wasOpenRef.current = true;
    } else if (wasOpenRef.current) {
      wasOpenRef.current = false;
      triggerRef.current?.focus();
    }
  }, [isOpen, triggerRef]);

  useEffect(() => {
    if (isOpen && position) searchInputRef.current?.focus();
  }, [isOpen, position]);

  const term = search.trim().toLowerCase();
  const matches = (...names: string[]) =>
    term === '' || names.some((name) => name.toLowerCase().includes(term));

  const categoryName = (category: { name_en: string; name_fr: string }) =>
    language === 'fr' ? category.name_fr : category.name_en;

  const visibleCategoryGroups = (categoryGroups ?? [])
    .map((group) => ({
      group,
      categories: (categories ?? []).filter(
        (category) =>
          category.group_id === group.id &&
          matches(category.name_en, category.name_fr),
      ),
    }))
    .filter((entry) => entry.categories.length > 0);

  const visibleMerchants = (merchants ?? []).filter((merchant) =>
    matches(merchant.name),
  );

  const visibleAccountTypes = ACCOUNT_TYPES.map((type) => ({
    type,
    accounts: (accounts ?? []).filter(
      (account) =>
        account.type === type &&
        matches(accountLabel(account), account.institution_name),
    ),
  })).filter((entry) => entry.accounts.length > 0);

  const untaggedLabel = t('transactions.filters.untagged');
  const showUntagged = matches(untaggedLabel);
  const visibleTags = (tags ?? []).filter((tag) => matches(tag.name));

  const toggleIds = (key: IdListKey, ids: number[], checked: boolean) =>
    setDraft((previous) => ({
      ...previous,
      [key]: setIds(previous[key], ids, checked),
    }));

  const { isAmountValid, isAmountMaxValid } = amountValidity(draft);
  const draftFilters = toFilters(draft);
  const draftCount = draftFilters ? countFilters(draftFilters) : null;

  const handleApply = () => {
    if (!draftFilters) return;
    navigate({
      search: (prev) => ({ ...prev, ...EMPTY_FILTERS, ...draftFilters }),
      replace: true,
    });
    setIsOpen(false);
  };

  const handleClearAll = () => {
    navigate({
      search: (prev) => ({ ...prev, ...EMPTY_FILTERS }),
      replace: true,
    });
    setIsOpen(false);
  };

  // Every open starts from what's currently applied — Cancel/outside click just drop the draft.
  const handleToggle = () => {
    if (!isOpen) {
      setDraft(toDraft(searchParams));
      setSection('categories');
      setSearch('');
    }
    setIsOpen((open) => !open);
  };

  const handleSectionChange = (next: Section) => {
    setSection(next);
    setSearch('');
  };

  const renderSelectAll = (key: IdListKey, ids: number[], extra?: boolean) => {
    const state = groupState(ids, draft[key]);
    // Tags' "Select all" also covers "Untagged" (`extra`), when it's visible.
    const checked =
      extra === undefined ? state.checked : state.checked && extra;
    const indeterminate =
      extra === undefined
        ? state.indeterminate
        : !checked && (state.checked || state.indeterminate || extra);
    return (
      <CheckboxRow
        variant="group"
        checked={checked}
        indeterminate={indeterminate}
        onChange={(isChecked) => {
          toggleIds(key, ids, isChecked);
          if (extra !== undefined) {
            setDraft((previous) => ({ ...previous, untagged: isChecked }));
          }
        }}
      >
        <span>{t('transactions.filters.selectAll')}</span>
      </CheckboxRow>
    );
  };

  const renderEmpty = () => (
    <p className="filters-popover-empty">
      {t('transactions.filters.noResults')}
    </p>
  );

  const renderList = () => {
    switch (section) {
      case 'categories': {
        if (visibleCategoryGroups.length === 0) return renderEmpty();
        const allIds = visibleCategoryGroups.flatMap((entry) =>
          entry.categories.map((category) => category.id),
        );
        return (
          <>
            {renderSelectAll('category_ids', allIds)}
            {visibleCategoryGroups.map(
              ({ group, categories: groupCategories }) => {
                const ids = groupCategories.map((category) => category.id);
                return (
                  <div key={group.id} className="filters-popover-group">
                    <CheckboxRow
                      variant="group"
                      {...groupState(ids, draft.category_ids)}
                      onChange={(checked) =>
                        toggleIds('category_ids', ids, checked)
                      }
                    >
                      <span>{categoryName(group)}</span>
                    </CheckboxRow>
                    {groupCategories.map((category) => (
                      <CheckboxRow
                        key={category.id}
                        checked={draft.category_ids.includes(category.id)}
                        onChange={(checked) =>
                          toggleIds('category_ids', [category.id], checked)
                        }
                      >
                        <span className="filters-popover-row-label">
                          {categoryName(category)}
                        </span>
                      </CheckboxRow>
                    ))}
                  </div>
                );
              },
            )}
          </>
        );
      }
      case 'merchants':
        if (visibleMerchants.length === 0) return renderEmpty();
        return visibleMerchants.map((merchant) => (
          <CheckboxRow
            key={merchant.id}
            checked={draft.merchant_ids.includes(merchant.id)}
            onChange={(checked) =>
              toggleIds('merchant_ids', [merchant.id], checked)
            }
          >
            <MerchantAvatar name={merchant.name} logoUrl={merchant.logo_url} />
            <span className="filters-popover-row-label">{merchant.name}</span>
            <span
              className="filters-popover-row-count"
              title={t('transactions.filters.transactionCount', {
                count: merchant.transaction_count,
              })}
            >
              <FontAwesomeIcon icon={faReceipt} />
              {merchant.transaction_count}
            </span>
          </CheckboxRow>
        ));
      case 'accounts': {
        if (visibleAccountTypes.length === 0) return renderEmpty();
        const allIds = visibleAccountTypes.flatMap((entry) =>
          entry.accounts.map((account) => account.id),
        );
        return (
          <>
            {renderSelectAll('account_ids', allIds)}
            {visibleAccountTypes.map(({ type, accounts: typeAccounts }) => {
              const ids = typeAccounts.map((account) => account.id);
              return (
                <div key={type} className="filters-popover-group">
                  <CheckboxRow
                    variant="group"
                    {...groupState(ids, draft.account_ids)}
                    onChange={(checked) =>
                      toggleIds('account_ids', ids, checked)
                    }
                  >
                    <span>{t(`settings.institutions.types.${type}`)}</span>
                  </CheckboxRow>
                  {typeAccounts.map((account) => (
                    <CheckboxRow
                      key={account.id}
                      checked={draft.account_ids.includes(account.id)}
                      onChange={(checked) =>
                        toggleIds('account_ids', [account.id], checked)
                      }
                    >
                      <LetterAvatar name={account.institution_name} />
                      <span
                        className="filters-popover-row-label"
                        title={`${account.institution_name} · ${accountLabel(account)}`}
                      >
                        {accountLabel(account)}
                      </span>
                    </CheckboxRow>
                  ))}
                </div>
              );
            })}
          </>
        );
      }
      case 'tags': {
        if (!showUntagged && visibleTags.length === 0) return renderEmpty();
        const ids = visibleTags.map((tag) => tag.id);
        return (
          <>
            {renderSelectAll(
              'tag_ids',
              ids,
              showUntagged ? draft.untagged : undefined,
            )}
            {showUntagged && (
              <CheckboxRow
                checked={draft.untagged}
                onChange={(checked) =>
                  setDraft((previous) => ({ ...previous, untagged: checked }))
                }
              >
                <span className="filters-popover-row-label">
                  {untaggedLabel}
                </span>
              </CheckboxRow>
            )}
            {visibleTags.map((tag) => (
              <CheckboxRow
                key={tag.id}
                checked={draft.tag_ids.includes(tag.id)}
                onChange={(checked) => toggleIds('tag_ids', [tag.id], checked)}
              >
                <TagDot color={tag.color} />
                <span className="filters-popover-row-label">{tag.name}</span>
              </CheckboxRow>
            ))}
          </>
        );
      }
      case 'amount':
        return (
          <div className="filters-popover-amount">
            <div className="filters-popover-group-name">
              {t('transactions.filters.sections.amount')}
            </div>
            {AMOUNT_OPS.map((op) => (
              <div key={op} className="filters-popover-amount-option">
                <label className="filters-popover-row filters-popover-row--item">
                  <input
                    type="radio"
                    name="filters-amount-op"
                    checked={draft.amount_op === op}
                    onChange={() =>
                      setDraft((previous) => ({ ...previous, amount_op: op }))
                    }
                  />
                  <span>{t(`transactions.filters.amountOps.${op}`)}</span>
                </label>
                {draft.amount_op === op && (
                  <div className="filters-popover-amount-fields">
                    <input
                      type="text"
                      inputMode="decimal"
                      className="filters-popover-input"
                      aria-label={t(`transactions.filters.amountOps.${op}`)}
                      placeholder={
                        op === 'between'
                          ? t('transactions.filters.amountMin')
                          : t('transactions.filters.amountPlaceholder')
                      }
                      value={draft.amount}
                      aria-invalid={!isAmountValid}
                      onChange={(event) => {
                        const amount = sanitizeAmountInput(event.target.value);
                        setDraft((previous) => ({ ...previous, amount }));
                      }}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter') handleApply();
                      }}
                    />
                    {op === 'between' && (
                      <input
                        type="text"
                        inputMode="decimal"
                        className="filters-popover-input"
                        aria-label={t('transactions.filters.amountMax')}
                        placeholder={t('transactions.filters.amountMax')}
                        value={draft.amount_max}
                        aria-invalid={!isAmountMaxValid}
                        onChange={(event) => {
                          const amountMax = sanitizeAmountInput(
                            event.target.value,
                          );
                          setDraft((previous) => ({
                            ...previous,
                            amount_max: amountMax,
                          }));
                        }}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter') handleApply();
                        }}
                      />
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        );
    }
  };

  // The summary column, one group per section with at least one selection, in nav order.
  const summary: {
    section: Section;
    items: SummaryItem[];
    onClear: () => void;
  }[] = [];
  const clearIds = (key: IdListKey) => () =>
    setDraft((previous) => ({ ...previous, [key]: [] }));
  const removeId = (key: IdListKey, id: number) => () =>
    toggleIds(key, [id], false);

  const categoryById = new Map((categories ?? []).map((c) => [c.id, c]));
  const categoryItems = draft.category_ids.flatMap((id) => {
    const category = categoryById.get(id);
    return category
      ? [
          {
            key: `c${id}`,
            label: categoryName(category),
            onRemove: removeId('category_ids', id),
          },
        ]
      : [];
  });
  if (categoryItems.length > 0) {
    summary.push({
      section: 'categories',
      items: categoryItems,
      onClear: clearIds('category_ids'),
    });
  }

  const merchantById = new Map((merchants ?? []).map((m) => [m.id, m]));
  const merchantItems = draft.merchant_ids.flatMap((id) => {
    const merchant = merchantById.get(id);
    return merchant
      ? [
          {
            key: `m${id}`,
            icon: (
              <MerchantAvatar
                name={merchant.name}
                logoUrl={merchant.logo_url}
              />
            ),
            label: merchant.name,
            onRemove: removeId('merchant_ids', id),
          },
        ]
      : [];
  });
  if (merchantItems.length > 0) {
    summary.push({
      section: 'merchants',
      items: merchantItems,
      onClear: clearIds('merchant_ids'),
    });
  }

  const accountById = new Map((accounts ?? []).map((a) => [a.id, a]));
  const accountItems = draft.account_ids.flatMap((id) => {
    const account = accountById.get(id);
    return account
      ? [
          {
            key: `a${id}`,
            icon: <LetterAvatar name={account.institution_name} />,
            label: accountLabel(account),
            onRemove: removeId('account_ids', id),
          },
        ]
      : [];
  });
  if (accountItems.length > 0) {
    summary.push({
      section: 'accounts',
      items: accountItems,
      onClear: clearIds('account_ids'),
    });
  }

  const tagById = new Map((tags ?? []).map((tag) => [tag.id, tag]));
  const tagItems: SummaryItem[] = [];
  if (draft.untagged) {
    tagItems.push({
      key: 'untagged',
      label: untaggedLabel,
      onRemove: () =>
        setDraft((previous) => ({ ...previous, untagged: false })),
    });
  }
  for (const id of draft.tag_ids) {
    const tag = tagById.get(id);
    if (tag) {
      tagItems.push({
        key: `t${id}`,
        icon: <TagDot color={tag.color} />,
        label: tag.name,
        onRemove: removeId('tag_ids', id),
      });
    }
  }
  if (tagItems.length > 0) {
    summary.push({
      section: 'tags',
      items: tagItems,
      onClear: () =>
        setDraft((previous) => ({ ...previous, tag_ids: [], untagged: false })),
    });
  }

  if (draftFilters?.amount !== undefined && draftFilters.amount_op) {
    const clearAmount = () =>
      setDraft((previous) => ({ ...previous, amount: '', amount_max: '' }));
    const amountLabel = t(
      `transactions.filters.amountSummary.${draftFilters.amount_op}`,
      {
        amount: formatAmount(draftFilters.amount, locale),
        max:
          draftFilters.amount_max !== undefined
            ? formatAmount(draftFilters.amount_max, locale)
            : '',
      },
    );
    summary.push({
      section: 'amount',
      items: [{ key: 'amount', label: amountLabel, onRemove: clearAmount }],
      onClear: clearAmount,
    });
  }

  return (
    <div className="filters-button-trigger">
      <TopMenuButton
        ref={triggerRef}
        icon={faFilter}
        label={
          activeCount > 0
            ? t('transactions.filters.triggerCount', { count: activeCount })
            : t('transactions.topMenu.filters')
        }
        aria-haspopup="dialog"
        aria-expanded={isOpen}
        onClick={handleToggle}
      />
      {activeCount > 0 && <span className="filters-button-badge" />}
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover filters-popover"
            ref={popoverRef}
            role="dialog"
            aria-label={t('transactions.topMenu.filters')}
            style={{ top: position.top, left: position.left }}
          >
            <div className="filters-popover-columns">
              <nav className="filters-popover-nav">
                <h4 className="filters-popover-title">
                  {t('transactions.topMenu.filters')}
                </h4>
                {SECTIONS.map((value) => (
                  <button
                    key={value}
                    type="button"
                    className={
                      value === section
                        ? 'filters-popover-nav-item filters-popover-nav-item--selected'
                        : 'filters-popover-nav-item'
                    }
                    aria-current={value === section}
                    onClick={() => handleSectionChange(value)}
                  >
                    {t(`transactions.filters.sections.${value}`)}
                  </button>
                ))}
              </nav>

              <div className="filters-popover-options">
                <div className="filters-popover-search">
                  <FontAwesomeIcon icon={faMagnifyingGlass} />
                  <input
                    ref={searchInputRef}
                    type="text"
                    value={search}
                    disabled={section === 'amount'}
                    placeholder={t('transactions.filters.searchPlaceholder')}
                    aria-label={t('transactions.filters.searchPlaceholder')}
                    onChange={(event) => setSearch(event.target.value)}
                  />
                </div>
                <div className="filters-popover-list">{renderList()}</div>
              </div>

              <div className="filters-popover-summary">
                <p className="filters-popover-summary-count">
                  {draftCount
                    ? t('transactions.filters.selectedCount', {
                        count: draftCount,
                      })
                    : t('transactions.filters.noneSelected')}
                </p>
                <div className="filters-popover-summary-list">
                  {summary.map((entry) => (
                    <div
                      key={entry.section}
                      className="filters-popover-summary-group"
                    >
                      <div className="filters-popover-summary-header">
                        <span>
                          {t(`transactions.filters.sections.${entry.section}`)}
                        </span>
                        <button
                          type="button"
                          className="filters-popover-link"
                          onClick={entry.onClear}
                        >
                          {t('transactions.filters.clear')}
                        </button>
                      </div>
                      {entry.items.map((item) => (
                        <div
                          key={item.key}
                          className="filters-popover-summary-item"
                        >
                          {item.icon}
                          <span
                            className="filters-popover-row-label"
                            title={item.label}
                          >
                            {item.label}
                          </span>
                          <button
                            type="button"
                            className="filters-popover-remove"
                            aria-label={t('transactions.filters.remove', {
                              name: item.label,
                            })}
                            onClick={item.onRemove}
                          >
                            <FontAwesomeIcon icon={faCircleXmark} />
                          </button>
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="filters-popover-actions">
              <button
                type="button"
                className="filters-popover-button"
                disabled={activeCount === 0}
                onClick={handleClearAll}
              >
                {t('transactions.filters.clear')}
              </button>
              <div className="filters-popover-actions-end">
                <button
                  type="button"
                  className="filters-popover-button"
                  onClick={() => setIsOpen(false)}
                >
                  {t('transactions.filters.cancel')}
                </button>
                <button
                  type="button"
                  className="filters-popover-button filters-popover-button--primary"
                  disabled={!draftFilters}
                  onClick={handleApply}
                >
                  {t('transactions.filters.apply')}
                </button>
              </div>
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
};
