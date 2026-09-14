import { useEffect, useMemo, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faPlus } from '@fortawesome/free-solid-svg-icons';
import { useCategories } from '../../categories/hooks/use-categories';
import { useCategoryGroups } from '../../categories/hooks/use-category-groups';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import { notImplementedToast } from '../../../lib/toast';
import type { Category } from '../../categories/utils/types';
import '../../../components/Popover.css';
import './CategoryPicker.css';

type CategoryPickerProps = {
  /** `null` when nothing is selected yet (e.g. the add-transaction form before a pick). */
  categoryId: number | null;
  label: string;
  className: string;
  onSelect: (categoryId: number) => void;
  /** Full accessible name for the trigger, e.g. "Category: Groceries" in a form where `label`
   * alone (just "Groceries") wouldn't say what the field is. Defaults to `label`. */
  ariaLabel?: string;
};

/** One arrow-key stop in the popover, in the order they're rendered: every visible category
 * across every group, then the "Create new category" action. */
type FlatItem = { type: 'category'; category: Category } | { type: 'create' };

const CREATE_OPTION_ID = 'category-picker-create-option';
const optionElementId = (item: FlatItem): string =>
  item.type === 'create'
    ? CREATE_OPTION_ID
    : `category-picker-option-${item.category.id}`;

export const CategoryPicker = ({
  categoryId,
  label,
  className,
  onSelect,
  ariaLabel,
}: CategoryPickerProps) => {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const { data: categories } = useCategories();
  const { data: categoryGroups } = useCategoryGroups();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const [search, setSearch] = useState('');
  const [highlightedIndex, setHighlightedIndex] = useState(0);

  const groups = useMemo(() => {
    if (!categories || !categoryGroups) return [];

    const term = search.trim().toLowerCase();
    const matches = (name: string) => name.toLowerCase().includes(term);

    return categoryGroups
      .map((group) => ({
        group,
        categories: categories.filter(
          (category) =>
            category.group_id === group.id &&
            (term === '' ||
              matches(category.name_en) ||
              matches(category.name_fr)),
        ),
      }))
      .filter(({ categories: groupCategories }) => groupCategories.length > 0);
  }, [categories, categoryGroups, search]);

  const hasResults = groups.length > 0;

  // Every arrow-key stop, in render order — categories grouped exactly like the JSX below, plus
  // "Create new category" always last — so an index here always means the same item on screen.
  const flatItems = useMemo<FlatItem[]>(() => {
    const items: FlatItem[] = groups.flatMap(
      ({ categories: groupCategories }) =>
        groupCategories.map((category) => ({
          type: 'category' as const,
          category,
        })),
    );
    items.push({ type: 'create' });
    return items;
  }, [groups]);

  const categoryIndexById = useMemo(() => {
    const map = new Map<number, number>();
    flatItems.forEach((item, index) => {
      if (item.type === 'category') map.set(item.category.id, index);
    });
    return map;
  }, [flatItems]);

  // A new search (or a fresh open) always re-highlights the top of the list, matching how a
  // typeahead combobox behaves elsewhere.
  useEffect(() => {
    setHighlightedIndex(0);
  }, [search, isOpen]);

  // Keeps the highlighted row in view as arrow keys move it past the edge of the (now
  // full-height, possibly long) scrollable list — the DOM focus itself never leaves the search
  // input, so the browser's own focus-follows-scroll doesn't handle this for us.
  useEffect(() => {
    if (!isOpen) return;
    const item = flatItems[highlightedIndex];
    if (!item) return;
    document
      .getElementById(optionElementId(item))
      ?.scrollIntoView({ block: 'nearest' });
  }, [isOpen, flatItems, highlightedIndex]);

  const handleSelect = (id: number) => {
    onSelect(id);
    setIsOpen(false);
  };

  const handleCreateNew = () => {
    notImplementedToast();
    setIsOpen(false);
  };

  const handleToggle = () => {
    setSearch('');
    setIsOpen((open) => !open);
  };

  const handleSearchKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setHighlightedIndex((index) => Math.min(index + 1, flatItems.length - 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setHighlightedIndex((index) => Math.max(index - 1, 0));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const item = flatItems[highlightedIndex];
      if (!item) return;
      if (item.type === 'create') handleCreateNew();
      else handleSelect(item.category.id);
    }
  };

  const highlightedItem = flatItems[highlightedIndex];

  return (
    <>
      <button
        type="button"
        className={`category-picker-trigger ${className}`}
        ref={triggerRef}
        onClick={handleToggle}
        aria-label={ariaLabel}
      >
        <span className="category-picker-trigger-label">{label}</span>
        <FontAwesomeIcon
          icon={faChevronDown}
          className="category-picker-trigger-chevron"
        />
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover category-picker-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <input
              type="text"
              className="category-picker-search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              onKeyDown={handleSearchKeyDown}
              placeholder={t('transactions.edit.categorySearchPlaceholder')}
              autoFocus
            />
            <div className="category-picker-list">
              {!hasResults && (
                <p className="category-picker-empty">
                  {t('transactions.edit.noCategoriesFound')}
                </p>
              )}
              {groups.map(({ group, categories: groupCategories }) => (
                <div key={group.id} className="category-picker-group">
                  <div className="category-picker-group-name">
                    {language === 'fr' ? group.name_fr : group.name_en}
                  </div>
                  {groupCategories.map((category) => {
                    const index = categoryIndexById.get(category.id);
                    const isHighlighted = index === highlightedIndex;
                    const isSelected = category.id === categoryId;
                    return (
                      <button
                        key={category.id}
                        id={optionElementId({ type: 'category', category })}
                        type="button"
                        className={[
                          'category-picker-option',
                          isSelected && 'category-picker-option--selected',
                          isHighlighted &&
                            'category-picker-option--highlighted',
                        ]
                          .filter(Boolean)
                          .join(' ')}
                        onClick={() => handleSelect(category.id)}
                      >
                        {language === 'fr'
                          ? category.name_fr
                          : category.name_en}
                      </button>
                    );
                  })}
                </div>
              ))}
            </div>
            <button
              id={CREATE_OPTION_ID}
              type="button"
              className={
                highlightedItem?.type === 'create'
                  ? 'category-picker-create category-picker-create--highlighted'
                  : 'category-picker-create'
              }
              onClick={handleCreateNew}
            >
              <FontAwesomeIcon icon={faPlus} />
              {t('transactions.edit.createCategory')}
            </button>
          </div>,
          document.body,
        )}
    </>
  );
};
