import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faTableColumns } from '@fortawesome/free-solid-svg-icons';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import type { ColumnVisibility, OptionalColumn } from '../utils/column-visibility';
import { OPTIONAL_COLUMNS } from '../utils/column-visibility';
import '../../../components/Popover.css';
import './ColumnsButton.css';

type ColumnsButtonProps = {
  visibility: ColumnVisibility;
  onToggle: (column: OptionalColumn) => void;
};

export const ColumnsButton = ({ visibility, onToggle }: ColumnsButtonProps) => {
  const { t } = useTranslation();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();

  const visibleCount = OPTIONAL_COLUMNS.filter((column) => visibility[column]).length;

  return (
    <>
      <button
        type="button"
        className="transactions-toolbar-button"
        ref={triggerRef}
        onClick={() => setIsOpen((open) => !open)}
      >
        <FontAwesomeIcon icon={faTableColumns} />
        <span>{t('transactions.toolbar.columns')}</span>
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover columns-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <p className="columns-popover-count">
              {t('transactions.columns.visibleCount', {
                visible: visibleCount,
                total: OPTIONAL_COLUMNS.length,
              })}
            </p>
            {OPTIONAL_COLUMNS.map((column) => (
              <label key={column} className="columns-popover-option">
                <span>{t(`transactions.columns.labels.${column}`)}</span>
                <input
                  type="checkbox"
                  role="switch"
                  className="columns-toggle-switch"
                  checked={visibility[column]}
                  onChange={() => onToggle(column)}
                />
              </label>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
};
