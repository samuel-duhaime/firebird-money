import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import '../features/transactions/components/AddTransactionModal.css';
import '../features/transactions/components/EditTransactionModal.css';
import './DangerModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type DangerModalProps = {
  title: string;
  /** What will happen, shown between the warning and the checkbox. */
  children: ReactNode;
  /** The checkbox the user ticks before the confirm button unlocks. */
  acknowledgement: string;
  confirmLabel: string;
  /** Another condition the confirm button waits for on top of the checkbox, like a download. */
  canConfirm?: boolean;
  isPending: boolean;
  onConfirm: () => void;
  onClose: () => void;
};

/**
 * A centered warning for an action that's hard or impossible to undo, like leaving or deleting a
 * household. The red confirm button stays disabled until the user ticks `acknowledgement`, so one
 * stray click never does it, and focus starts on Cancel rather than on the dangerous button.
 */
export const DangerModal = ({
  title,
  children,
  acknowledgement,
  confirmLabel,
  canConfirm = true,
  isPending,
  onConfirm,
  onClose,
}: DangerModalProps) => {
  const { t } = useTranslation();
  const [acknowledged, setAcknowledged] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  // The button that opened this: closing removes whatever is focused in here, so focus goes back
  // to it instead of being lost.
  const [returnFocusTarget] = useState<HTMLElement | null>(
    () => document.activeElement as HTMLElement | null,
  );

  const handleClose = () => {
    onClose();
    returnFocusTarget?.focus();
  };

  useEffect(() => {
    cancelRef.current?.focus();
  }, []);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      handleClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    ).filter(
      (element) => element.tabIndex !== -1 && !element.matches(':disabled'),
    );
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        ref={dialogRef}
        className="danger-modal"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="danger-modal-title"
        aria-describedby="danger-modal-body"
        onKeyDown={handleKeyDown}
      >
        <div className="danger-modal-header">
          <span className="danger-modal-icon" aria-hidden="true">
            <FontAwesomeIcon icon={faTriangleExclamation} />
          </span>
          <h2 id="danger-modal-title">{title}</h2>
        </div>

        <div id="danger-modal-body" className="modal-body danger-modal-body">
          {children}

          <label className="danger-modal-acknowledgement">
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(e) => setAcknowledged(e.target.checked)}
            />
            {acknowledgement}
          </label>
        </div>

        <div className="modal-footer">
          <div className="modal-footer-actions">
            <button
              ref={cancelRef}
              type="button"
              className="modal-button modal-button--cancel"
              onClick={handleClose}
            >
              {t('transactions.add.cancel')}
            </button>
            <button
              type="button"
              className="modal-button modal-button--delete-confirm"
              onClick={onConfirm}
              disabled={!acknowledged || !canConfirm || isPending}
            >
              {confirmLabel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
