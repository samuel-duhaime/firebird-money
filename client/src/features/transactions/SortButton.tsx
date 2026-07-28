import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown } from '@fortawesome/free-solid-svg-icons';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import type { SortOrder } from './types';
import '../../components/Popover.css';
import './SortButton.css';

const routeApi = getRouteApi('/transactions');

const SORT_ORDERS: SortOrder[] = ['date', 'inverse_date', 'amount', 'inverse_amount'];

export const SortButton = () => {
  const { t } = useTranslation();
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
        <span>{t('transactions.sort.trigger')}</span>
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
            {SORT_ORDERS.map((value) => (
              <button
                key={value}
                type="button"
                className={
                  value === selected
                    ? 'sort-popover-option sort-popover-option--selected'
                    : 'sort-popover-option'
                }
                onClick={() => handleSelect(value)}
              >
                {t(`transactions.sort.options.${value}`)}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
};
