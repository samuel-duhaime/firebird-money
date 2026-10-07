import { useEffect, useMemo, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faPlus } from '@fortawesome/free-solid-svg-icons';
import { useMerchants } from '../../merchants/hooks/use-merchants';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import type { Merchant } from '../../merchants/utils/types';
import '../../../components/Popover.css';
import './MerchantPicker.css';

type MerchantPickerProps = {
  /** `null` when nothing is selected yet (e.g. the add-transaction form before a pick). */
  merchantId: number | null;
  label: string;
  className: string;
  onSelect: (merchantId: number) => void;
  /** Full accessible name for the trigger, e.g. "Merchant: Starbucks" in a form where `label`
   * alone (just "Starbucks") wouldn't say what the field is. Defaults to `label`. */
  ariaLabel?: string;
};

/** One arrow-key stop in the popover, in the order they're rendered: every visible merchant (no
 * grouping — merchants are flat, unlike categories), then the "Create new merchant" action. */
type FlatItem = { type: 'merchant'; merchant: Merchant } | { type: 'create' };

const CREATE_OPTION_ID = 'merchant-picker-create-option';
const optionElementId = (item: FlatItem): string =>
  item.type === 'create'
    ? CREATE_OPTION_ID
    : `merchant-picker-option-${item.merchant.id}`;

export const MerchantPicker = ({
  merchantId,
  label,
  className,
  onSelect,
  ariaLabel,
}: MerchantPickerProps) => {
  const { t } = useTranslation();
  const { data: merchants } = useMerchants();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const [search, setSearch] = useState('');
  const [highlightedIndex, setHighlightedIndex] = useState(0);

  const filtered = useMemo(() => {
    if (!merchants) return [];
    const term = search.trim().toLowerCase();
    if (term === '') return merchants;
    return merchants.filter((merchant) =>
      merchant.name.toLowerCase().includes(term),
    );
  }, [merchants, search]);

  const hasResults = filtered.length > 0;

  // Every arrow-key stop, in render order — the filtered merchants exactly as rendered below,
  // plus "Create new merchant" always last — so an index here always means the same item on
  // screen.
  const flatItems = useMemo<FlatItem[]>(() => {
    const items: FlatItem[] = filtered.map((merchant) => ({
      type: 'merchant' as const,
      merchant,
    }));
    items.push({ type: 'create' });
    return items;
  }, [filtered]);

  const merchantIndexById = useMemo(() => {
    const map = new Map<number, number>();
    flatItems.forEach((item, index) => {
      if (item.type === 'merchant') map.set(item.merchant.id, index);
    });
    return map;
  }, [flatItems]);

  // A new search (or a fresh open) always re-highlights the top of the list, matching how a
  // typeahead combobox behaves elsewhere (see CategoryPicker).
  useEffect(() => {
    setHighlightedIndex(0);
  }, [search, isOpen]);

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
    setIsOpen(false);
    // Opens in a new tab rather than navigating away, so whatever the caller is in the middle of
    // here (e.g. an in-progress add-transaction form) stays open behind it.
    window.open('/settings/merchants', '_blank', 'noopener');
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
      else handleSelect(item.merchant.id);
    }
  };

  const highlightedItem = flatItems[highlightedIndex];

  return (
    <>
      <button
        type="button"
        className={`merchant-picker-trigger ${className}`}
        ref={triggerRef}
        onClick={handleToggle}
        aria-label={ariaLabel}
      >
        <span className="merchant-picker-trigger-label">{label}</span>
        <FontAwesomeIcon
          icon={faChevronDown}
          className="merchant-picker-trigger-chevron"
        />
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover merchant-picker-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <input
              type="text"
              className="merchant-picker-search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              onKeyDown={handleSearchKeyDown}
              placeholder={t('transactions.edit.merchantSearchPlaceholder')}
              autoFocus
            />
            <div className="merchant-picker-list">
              {!hasResults && (
                <p className="merchant-picker-empty">
                  {t('transactions.edit.noMerchantsFound')}
                </p>
              )}
              {filtered.map((merchant) => {
                const index = merchantIndexById.get(merchant.id);
                const isHighlighted = index === highlightedIndex;
                const isSelected = merchant.id === merchantId;
                return (
                  <button
                    key={merchant.id}
                    id={optionElementId({ type: 'merchant', merchant })}
                    type="button"
                    className={[
                      'merchant-picker-option',
                      isSelected && 'merchant-picker-option--selected',
                      isHighlighted && 'merchant-picker-option--highlighted',
                    ]
                      .filter(Boolean)
                      .join(' ')}
                    onClick={() => handleSelect(merchant.id)}
                  >
                    {merchant.name}
                  </button>
                );
              })}
            </div>
            <button
              id={CREATE_OPTION_ID}
              type="button"
              className={
                highlightedItem?.type === 'create'
                  ? 'merchant-picker-create merchant-picker-create--highlighted'
                  : 'merchant-picker-create'
              }
              onClick={handleCreateNew}
            >
              <FontAwesomeIcon icon={faPlus} />
              {t('transactions.edit.createMerchant')}
            </button>
          </div>,
          document.body,
        )}
    </>
  );
};
