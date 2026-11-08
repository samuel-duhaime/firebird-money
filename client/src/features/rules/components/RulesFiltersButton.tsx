import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faCircleXmark,
  faFilter,
  faMagnifyingGlass,
} from '@fortawesome/free-solid-svg-icons';
import type { MultiSelectOption } from '../../../components/MultiSelect';
import { TopMenuButton } from '../../../components/TopMenuButton';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import { CheckboxRow } from '../../transactions/components/FiltersButton';
import { useRuleLookups } from '../hooks/use-rule-lookups';
import { activeFilterCount, NO_RULE_FILTERS } from '../utils/filter';
import type { RuleFilters } from '../utils/filter';
import '../../../components/Popover.css';
import '../../transactions/components/FiltersButton.css';

type Section = 'categories' | 'merchants' | 'accounts' | 'owners' | 'tags';

const SECTIONS: Section[] = [
  'categories',
  'merchants',
  'accounts',
  'owners',
  'tags',
];

const SECTION_KEYS = {
  categories: 'categoryIds',
  merchants: 'merchantIds',
  accounts: 'accountIds',
  owners: 'owners',
  tags: 'tagIds',
} as const satisfies Record<Section, keyof RuleFilters>;

type Value = RuleFilters[keyof RuleFilters][number];

/** Adds `values` to `list` when `checked`, otherwise removes them — keeping `list`'s order. */
const setValues = (list: Value[], values: Value[], checked: boolean) =>
  checked
    ? [...list, ...values.filter((value) => !list.includes(value))]
    : list.filter((value) => !values.includes(value));

/** Checkbox state over `values` against the `selected` list. */
const groupState = (values: Value[], selected: Value[]) => {
  const count = values.filter((value) => selected.includes(value)).length;
  return {
    checked: values.length > 0 && count === values.length,
    indeterminate: count > 0 && count < values.length,
  };
};

/** The rules page's Filters button, laid out like the transactions page's: pick merchants,
 * categories, accounts, owners, or tags, and keep the rules that name any of them. */
export const RulesFiltersButton = ({
  filters,
  onChange,
}: {
  filters: RuleFilters;
  onChange: (filters: RuleFilters) => void;
}) => {
  const { t } = useTranslation();
  const lookups = useRuleLookups();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [section, setSection] = useState<Section>('categories');
  const [search, setSearch] = useState('');
  const [draft, setDraft] = useState<RuleFilters>(filters);

  const activeCount = activeFilterCount(filters);
  const draftCount = activeFilterCount(draft);

  useEffect(() => {
    if (isOpen && position) searchInputRef.current?.focus();
  }, [isOpen, position]);

  const optionsBySection: Record<Section, MultiSelectOption<Value>[]> = {
    categories: lookups.categoryOptions,
    merchants: lookups.merchantOptions,
    accounts: lookups.accountOptions,
    owners: lookups.ownerOptions,
    tags: lookups.tagOptions,
  };

  const selectedIn = (key: Section): Value[] => draft[SECTION_KEYS[key]];
  const toggle = (key: Section, values: Value[], checked: boolean) =>
    setDraft((previous) => ({
      ...previous,
      [SECTION_KEYS[key]]: setValues(
        previous[SECTION_KEYS[key]],
        values,
        checked,
      ),
    }));
  const clear = (key: Section) =>
    setDraft((previous) => ({ ...previous, [SECTION_KEYS[key]]: [] }));

  // Every open starts from what's currently applied — Cancel/outside click just drop the draft.
  const handleToggle = () => {
    if (!isOpen) {
      setDraft(filters);
      setSection('categories');
      setSearch('');
    }
    setIsOpen((open) => !open);
  };

  const handleApply = () => {
    onChange(draft);
    setIsOpen(false);
  };

  const handleClearAll = () => {
    onChange(NO_RULE_FILTERS);
    setIsOpen(false);
  };

  const renderList = () => {
    const term = search.trim().toLowerCase();
    const visible = optionsBySection[section].filter(
      (option) => term === '' || option.label.toLowerCase().includes(term),
    );
    if (visible.length === 0) {
      return (
        <p className="filters-popover-empty">
          {t('transactions.filters.noResults')}
        </p>
      );
    }
    const selected = selectedIn(section);
    const row = (option: MultiSelectOption<Value>) => (
      <CheckboxRow
        key={option.value}
        checked={selected.includes(option.value)}
        onChange={(checked) => toggle(section, [option.value], checked)}
      >
        {option.icon}
        <span className="filters-popover-row-label" title={option.label}>
          {option.label}
        </span>
      </CheckboxRow>
    );
    const groups: { name: string; options: MultiSelectOption<Value>[] }[] = [];
    for (const option of visible) {
      const last = groups[groups.length - 1];
      if (option.group && last?.name === option.group)
        last.options.push(option);
      else if (option.group)
        groups.push({ name: option.group, options: [option] });
    }
    const allValues = visible.map((option) => option.value);

    return (
      <>
        {section !== 'merchants' && (
          <CheckboxRow
            variant="group"
            {...groupState(allValues, selected)}
            onChange={(checked) => toggle(section, allValues, checked)}
          >
            <span>{t('transactions.filters.selectAll')}</span>
          </CheckboxRow>
        )}
        {groups.length > 0
          ? groups.map((group) => {
              const values = group.options.map((option) => option.value);
              return (
                <div key={group.name} className="filters-popover-group">
                  <CheckboxRow
                    variant="group"
                    {...groupState(values, selected)}
                    onChange={(checked) => toggle(section, values, checked)}
                  >
                    <span>{group.name}</span>
                  </CheckboxRow>
                  {group.options.map(row)}
                </div>
              );
            })
          : visible.map(row)}
      </>
    );
  };

  const summary = SECTIONS.flatMap((key) => {
    const selected = selectedIn(key);
    if (selected.length === 0) return [];
    const options = optionsBySection[key];
    const items = selected.map((value) => {
      const option = options.find((o) => o.value === value);
      return {
        value,
        icon: option?.icon,
        label: option?.label ?? t('settings.rules.deleted'),
      };
    });
    return [{ key, items }];
  });

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
                    onClick={() => {
                      setSection(value);
                      setSearch('');
                    }}
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
                    placeholder={t('transactions.filters.searchPlaceholder')}
                    aria-label={t('transactions.filters.searchPlaceholder')}
                    onChange={(event) => setSearch(event.target.value)}
                  />
                </div>
                <div className="filters-popover-list">{renderList()}</div>
              </div>

              <div className="filters-popover-summary">
                <p className="filters-popover-summary-count">
                  {draftCount > 0
                    ? t('transactions.filters.selectedCount', {
                        count: draftCount,
                      })
                    : t('transactions.filters.noneSelected')}
                </p>
                <div className="filters-popover-summary-list">
                  {summary.map((entry) => (
                    <div
                      key={entry.key}
                      className="filters-popover-summary-group"
                    >
                      <div className="filters-popover-summary-header">
                        <span>
                          {t(`transactions.filters.sections.${entry.key}`)}
                        </span>
                        <button
                          type="button"
                          className="filters-popover-link"
                          onClick={() => clear(entry.key)}
                        >
                          {t('transactions.filters.clear')}
                        </button>
                      </div>
                      {entry.items.map((item) => (
                        <div
                          key={item.value}
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
                            onClick={() =>
                              toggle(entry.key, [item.value], false)
                            }
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
                disabled={activeCount === 0 && draftCount === 0}
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
