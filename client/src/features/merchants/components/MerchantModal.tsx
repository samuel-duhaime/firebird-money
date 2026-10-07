import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import { useCreateMerchant } from '../hooks/use-create-merchant';
import { useUpdateMerchant } from '../hooks/use-update-merchant';
import type { Merchant } from '../utils/types';
import '../../transactions/components/AddTransactionModal.css';
import './MerchantModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type MerchantModalProps = {
  /** A merchant the household owns, to edit — `undefined` creates a new one. Common merchants
   * (shared across every household) are never passed in here; see `MerchantsList`. */
  merchant?: Merchant;
  onClose: () => void;
};

export const MerchantModal = ({ merchant, onClose }: MerchantModalProps) => {
  const { t } = useTranslation();
  const [name, setName] = useState(merchant?.name ?? '');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const firstFieldRef = useRef<HTMLInputElement>(null);

  const createMerchantMutation = useCreateMerchant();
  const updateMerchantMutation = useUpdateMerchant();
  const isPending =
    createMerchantMutation.isPending || updateMerchantMutation.isPending;

  useEffect(() => {
    firstFieldRef.current?.focus();
  }, []);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
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

  const handleNameChange = (e: ChangeEvent<HTMLInputElement>) => {
    setError('');
    setName(e.target.value);
  };

  const handleSubmit = () => {
    if (!name.trim()) {
      setError(t('settings.merchants.modal.required'));
      return;
    }

    const onError = (mutationError: unknown) => {
      setError(
        mutationError instanceof ApiError && mutationError.status === 409
          ? t('settings.merchants.modal.duplicateName')
          : t('settings.merchants.modal.failed'),
      );
    };

    if (merchant) {
      updateMerchantMutation.mutate(
        { id: merchant.id, patch: { name: name.trim() } },
        { onSuccess: onClose, onError },
      );
    } else {
      createMerchantMutation.mutate(
        { name: name.trim() },
        { onSuccess: onClose, onError },
      );
    }
  };

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="merchant-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="merchant-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="merchant-modal-title">
            {merchant
              ? t('settings.merchants.modal.editTitle')
              : t('settings.merchants.modal.newTitle')}
          </h2>

          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('transactions.add.close', 'Close')}
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <label htmlFor="merchant-name">
            {t('settings.merchants.modal.name')}
          </label>
          <input
            ref={firstFieldRef}
            id="merchant-name"
            type="text"
            value={name}
            onChange={handleNameChange}
          />

          {error && (
            <p className="modal-error" role="alert">
              {error}
            </p>
          )}
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="modal-button modal-button--cancel"
            onClick={onClose}
          >
            {t('settings.merchants.modal.cancel')}
          </button>

          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
            disabled={isPending}
          >
            {t('settings.merchants.modal.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
