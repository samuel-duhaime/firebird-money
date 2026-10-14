import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import { useUpdateInstitution } from '../hooks/use-update-institution';
import type { Institution } from '../utils/types';
import '../../transactions/components/AddTransactionModal.css';
import '../../accounts/components/AccountModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type InstitutionModalProps = {
  /** A custom institution this household owns — common ones can't be edited. */
  institution: Institution;
  onClose: () => void;
};

export const InstitutionModal = ({
  institution,
  onClose,
}: InstitutionModalProps) => {
  const { t } = useTranslation();
  const [name, setName] = useState(institution.name);
  const [website, setWebsite] = useState(institution.website_url ?? '');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const firstFieldRef = useRef<HTMLInputElement>(null);
  const updateInstitutionMutation = useUpdateInstitution();

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

  const handleSubmit = () => {
    if (!name.trim()) {
      setError(t('settings.institutions.modal.nameRequired'));
      return;
    }
    updateInstitutionMutation.mutate(
      {
        id: institution.id,
        patch: { name: name.trim(), website_url: website.trim() },
      },
      {
        onSuccess: onClose,
        onError: (mutationError) => {
          setError(
            mutationError instanceof ApiError && mutationError.status === 409
              ? t('settings.institutions.modal.duplicateInstitution')
              : mutationError instanceof ApiError &&
                  mutationError.status === 400
                ? t('settings.institutions.modal.invalidWebsite')
                : t('settings.institutions.modal.failed'),
          );
        },
      },
    );
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
        className="account-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="institution-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="institution-modal-title">
            {t('settings.institutions.modal.editInstitutionTitle')}
          </h2>
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('transactions.add.close')}
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <label htmlFor="institution-name">
            {t('settings.institutions.modal.institutionName')}
          </label>
          <input
            ref={firstFieldRef}
            id="institution-name"
            type="text"
            value={name}
            onChange={(e) => {
              setError('');
              setName(e.target.value);
            }}
          />
          <label htmlFor="institution-website">
            {t('settings.institutions.modal.website')}
          </label>
          <input
            id="institution-website"
            type="url"
            placeholder="https://"
            value={website}
            onChange={(e) => {
              setError('');
              setWebsite(e.target.value);
            }}
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
            {t('settings.institutions.modal.cancel')}
          </button>
          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSubmit}
            disabled={updateInstitutionMutation.isPending}
          >
            {t('settings.institutions.modal.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
