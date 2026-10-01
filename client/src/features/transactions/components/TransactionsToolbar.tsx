import { useEffect, useRef } from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useTranslation } from 'react-i18next';
import { faChevronDown, faSquareCheck } from '@fortawesome/free-solid-svg-icons';
import { notImplementedToast } from '../../../lib/toast';
import { SortButton } from './SortButton';
import { ColumnsButton } from './ColumnsButton';
import type { ColumnVisibility, OptionalColumn } from '../utils/column-visibility';
import './TransactionsToolbar.css';

type TransactionsToolbarProps = {
  selectionMode: boolean;
  selectedCount: number;
  totalCount: number;
  columnVisibility: ColumnVisibility;
  onEnterSelectionMode: () => void;
  onCancelSelection: () => void;
  onToggleSelectAll: () => void;
  onOpenBulkEdit: () => void;
  onToggleColumn: (column: OptionalColumn) => void;
};

export const TransactionsToolbar = ({
  selectionMode,
  selectedCount,
  totalCount,
  columnVisibility,
  onEnterSelectionMode,
  onCancelSelection,
  onToggleSelectAll,
  onOpenBulkEdit,
  onToggleColumn,
}: TransactionsToolbarProps) => {
  const { t } = useTranslation();
  const selectAllRef = useRef<HTMLInputElement>(null);
  const allSelected = totalCount > 0 && selectedCount === totalCount;

  // The "some but not all selected" state has no HTML attribute — it's a DOM property that has to
  // be set imperatively on the checkbox element itself.
  useEffect(() => {
    if (selectAllRef.current) {
      selectAllRef.current.indeterminate = selectedCount > 0 && !allSelected;
    }
  }, [selectedCount, allSelected]);

  return (
    <div className="transactions-toolbar">
      {selectionMode ? (
        <label className="transactions-toolbar-select-all">
          <input
            ref={selectAllRef}
            type="checkbox"
            checked={allSelected}
            onChange={onToggleSelectAll}
          />
          <span>
            {selectedCount > 0
              ? t('transactions.toolbar.selected', { count: selectedCount })
              : t('transactions.toolbar.selectAllHint')}
          </span>
        </label>
      ) : (
        <button
          type="button"
          className="transactions-toolbar-button"
          onClick={notImplementedToast}
        >
          <span>{t('transactions.toolbar.allTransactions')}</span>
          <FontAwesomeIcon icon={faChevronDown} />
        </button>
      )}
      <div className="transactions-toolbar-actions">
        {selectionMode ? (
          <>
            <button
              type="button"
              className="transactions-toolbar-button"
              onClick={onCancelSelection}
            >
              <span>{t('transactions.toolbar.cancel')}</span>
            </button>
            <button
              type="button"
              className="transactions-toolbar-button transactions-toolbar-button--primary"
              onClick={onOpenBulkEdit}
              disabled={selectedCount === 0}
            >
              <span>
                {t('transactions.toolbar.editCount', { count: selectedCount })}
              </span>
            </button>
          </>
        ) : (
          <button
            type="button"
            className="transactions-toolbar-button"
            onClick={onEnterSelectionMode}
          >
            <FontAwesomeIcon icon={faSquareCheck} />
            <span>{t('transactions.toolbar.editMultiple')}</span>
          </button>
        )}
        <span className="transactions-toolbar-divider" />
        <SortButton />
        <ColumnsButton visibility={columnVisibility} onToggle={onToggleColumn} />
      </div>
    </div>
  );
};
