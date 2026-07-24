import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown } from '@fortawesome/free-solid-svg-icons';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import type { SortOrder } from './types';
import '../../components/Popover.css';
import './SortButton.css';

const routeApi = getRouteApi('/transactions');

const SORT_OPTIONS: { value: SortOrder; label: string }[] = [
  { value: 'date', label: 'Date (new to old)' },
  { value: 'inverse_date', label: 'Date (old to new)' },
  { value: 'amount', label: 'Amount (high to low)' },
  { value: 'inverse_amount', label: 'Amount (low to high)' },
];

export const SortButton = () => {
  const { order } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();

  const selected = order ?? 'date';
  const isActive = selected !== 'date';

  const handleSelect = (value: SortOrder) => {
    navigate({ search: (prev) => ({ ...prev, order: value }), replace: true });
    setIsOpen(false);
  };

  return (
    <>
      <button
        type="button"
        className="transactions-toolbar-button sort-button-trigger"
        ref={triggerRef}
        onClick={() => setIsOpen((open) => !open)}
      >
        <span>Sort</span>
        <FontAwesomeIcon icon={faChevronDown} />
        {isActive && <span className="sort-button-badge" />}
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover sort-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            {SORT_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                className={
                  option.value === selected
                    ? 'sort-popover-option sort-popover-option--selected'
                    : 'sort-popover-option'
                }
                onClick={() => handleSelect(option.value)}
              >
                {option.label}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
};
