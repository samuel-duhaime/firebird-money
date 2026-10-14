import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faArrowLeft } from '@fortawesome/free-solid-svg-icons';
import { ApiError } from '../../../lib/api-client';
import { useInstitutions } from '../../institutions/hooks/use-institutions';
import { useCreateInstitution } from '../../institutions/hooks/use-create-institution';
import type { Institution } from '../../institutions/utils/types';
import { useCreateAccount } from '../hooks/use-create-account';
import { useUpdateAccount } from '../hooks/use-update-account';
import { ACCOUNT_TYPES } from '../utils/types';
import type { Account, AccountType } from '../utils/types';
import { InstitutionAvatar } from '../../institutions/components/InstitutionAvatar';
import { Select } from '../../../components/Select';
import '../../transactions/components/AddTransactionModal.css';
import './AccountModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

const CURRENCIES = ['CAD', 'USD'];

/** `example.com` from `https://www.example.com/ca`, for the picker's second line. */
const hostname = (url: string | null) => {
  if (!url) return '';
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
};

/** Case- and accent-insensitive, so "banque nationale" finds "Banque Nationale" and "Montreal"
 * finds "Montréal". */
const normalize = (text: string) =>
  text.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

type Step = 'institution' | 'customInstitution' | 'account';

type AccountModalProps = {
  /** The account to edit — `undefined` adds a new one. */
  account?: Account;
  /** When adding, skip the institution step and add straight into this one. */
  institutionId?: number;
  onClose: () => void;
};

/**
 * Adding: pick an institution (searchable list, or add a custom one), then fill in the account.
 * Editing: straight to the account form, where the institution is a plain select.
 */
export const AccountModal = ({
  account,
  institutionId: initialInstitutionId,
  onClose,
}: AccountModalProps) => {
  const { t } = useTranslation();
  const { data: institutions } = useInstitutions();
  const [step, setStep] = useState<Step>(
    account || initialInstitutionId ? 'account' : 'institution',
  );
  const [search, setSearch] = useState('');
  const [institutionId, setInstitutionId] = useState<number | null>(
    account?.institution_id ?? initialInstitutionId ?? null,
  );
  const [customName, setCustomName] = useState('');
  const [customWebsite, setCustomWebsite] = useState('');
  const [name, setName] = useState(account?.name ?? '');
  const [type, setType] = useState<AccountType>(account?.type ?? 'checking');
  const [lastFour, setLastFour] = useState(account?.last_four ?? '');
  const [currency, setCurrency] = useState(account?.currency ?? 'CAD');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);

  const createInstitutionMutation = useCreateInstitution();
  const createAccountMutation = useCreateAccount();
  const updateAccountMutation = useUpdateAccount();
  const isPending =
    createInstitutionMutation.isPending ||
    createAccountMutation.isPending ||
    updateAccountMutation.isPending;

  // Each step's first field takes focus as the step opens.
  useEffect(() => {
    dialogRef.current?.querySelector<HTMLElement>('[data-autofocus]')?.focus();
  }, [step]);

  const filteredInstitutions = useMemo(() => {
    const term = normalize(search.trim());
    if (!term) return institutions ?? [];
    return (institutions ?? []).filter((institution) =>
      [institution.name, ...institution.aliases].some((candidate) =>
        normalize(candidate).includes(term),
      ),
    );
  }, [institutions, search]);

  const selectedInstitution = institutions?.find((i) => i.id === institutionId);

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

  const goTo = (next: Step) => {
    setError('');
    setStep(next);
  };

  const handlePickInstitution = (institution: Institution) => {
    setInstitutionId(institution.id);
    goTo('account');
  };

  const handleCreateInstitution = () => {
    if (!customName.trim()) {
      setError(t('settings.institutions.modal.nameRequired'));
      return;
    }
    createInstitutionMutation.mutate(
      {
        name: customName.trim(),
        website_url: customWebsite.trim() || undefined,
      },
      {
        onSuccess: (institution) => {
          setInstitutionId(institution.id);
          goTo('account');
        },
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

  const handleSaveAccount = () => {
    if (institutionId === null) return;
    if (!name.trim()) {
      setError(t('settings.institutions.modal.nameRequired'));
      return;
    }
    if (!/^[0-9A-Za-z]{0,4}$/.test(lastFour.trim())) {
      setError(t('settings.institutions.modal.invalidLastFour'));
      return;
    }

    const fields = {
      institution_id: institutionId,
      name: name.trim(),
      type,
      last_four: lastFour.trim(),
      currency,
    };
    const onError = () => setError(t('settings.institutions.modal.failed'));

    if (account) {
      updateAccountMutation.mutate(
        { id: account.id, patch: fields },
        { onSuccess: onClose, onError },
      );
    } else {
      createAccountMutation.mutate(
        { ...fields, last_four: fields.last_four || undefined },
        { onSuccess: onClose, onError },
      );
    }
  };

  const title =
    step === 'institution'
      ? t('settings.institutions.modal.chooseInstitution')
      : step === 'customInstitution'
        ? t('settings.institutions.modal.customInstitutionTitle')
        : account
          ? t('settings.institutions.modal.editTitle')
          : t('settings.institutions.modal.newTitle');

  // Back goes one step up the add flow; there's nowhere to go back to when editing, or when the
  // institution was chosen before the modal opened.
  const backStep: Step | null =
    step === 'customInstitution'
      ? 'institution'
      : step === 'account' && !account && !initialInstitutionId
        ? 'institution'
        : null;

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
        aria-labelledby="account-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <div className="account-modal-title">
            {backStep && (
              <button
                type="button"
                className="account-modal-back"
                onClick={() => goTo(backStep)}
                aria-label={t('settings.institutions.modal.back')}
              >
                <FontAwesomeIcon icon={faArrowLeft} />
              </button>
            )}
            <h2 id="account-modal-title">{title}</h2>
          </div>

          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('transactions.add.close')}
          >
            ×
          </button>
        </div>

        {step === 'institution' && (
          <div className="modal-body">
            <input
              data-autofocus
              type="search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t('settings.institutions.modal.searchInstitutions', {
                count: institutions?.length ?? 0,
              })}
              aria-label={t('settings.institutions.modal.searchLabel')}
            />

            <ul className="account-modal-institutions">
              {filteredInstitutions.map((institution) => (
                <li key={institution.id}>
                  <button
                    type="button"
                    className="account-modal-institution"
                    onClick={() => handlePickInstitution(institution)}
                  >
                    <InstitutionAvatar name={institution.name} />
                    <span className="account-modal-institution-text">
                      <span className="account-modal-institution-name">
                        {institution.name}
                      </span>
                      {institution.website_url && (
                        <span className="account-modal-institution-url">
                          {hostname(institution.website_url)}
                        </span>
                      )}
                    </span>
                  </button>
                </li>
              ))}
              {filteredInstitutions.length === 0 && (
                <li className="account-modal-empty">
                  {t('settings.institutions.modal.noInstitutionsFound')}
                </li>
              )}
            </ul>

            <button
              type="button"
              className="modal-button modal-button--cancel account-modal-custom"
              onClick={() => {
                setCustomName(search.trim());
                goTo('customInstitution');
              }}
            >
              {t('settings.institutions.modal.addCustomInstitution')}
            </button>
          </div>
        )}

        {step === 'customInstitution' && (
          <>
            <div className="modal-body">
              <p className="account-modal-hint">
                {t('settings.institutions.modal.customInstitutionHint')}
              </p>
              <label htmlFor="custom-institution-name">
                {t('settings.institutions.modal.institutionName')}
              </label>
              <input
                data-autofocus
                id="custom-institution-name"
                type="text"
                value={customName}
                onChange={(e) => {
                  setError('');
                  setCustomName(e.target.value);
                }}
              />
              <label htmlFor="custom-institution-website">
                {t('settings.institutions.modal.website')}
              </label>
              <input
                id="custom-institution-website"
                type="url"
                placeholder="https://"
                value={customWebsite}
                onChange={(e) => {
                  setError('');
                  setCustomWebsite(e.target.value);
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
                onClick={handleCreateInstitution}
                disabled={isPending}
              >
                {t('settings.institutions.modal.continue')}
              </button>
            </div>
          </>
        )}

        {step === 'account' && (
          <>
            <div className="modal-body">
              {account ? (
                <>
                  <label htmlFor="account-institution">
                    {t('settings.institutions.modal.institution')}
                  </label>
                  <Select
                    id="account-institution"
                    value={institutionId ?? ''}
                    onChange={(e) => setInstitutionId(Number(e.target.value))}
                  >
                    {institutions?.map((institution) => (
                      <option key={institution.id} value={institution.id}>
                        {institution.name}
                      </option>
                    ))}
                  </Select>
                </>
              ) : (
                selectedInstitution && (
                  <div className="account-modal-selected-institution">
                    <InstitutionAvatar name={selectedInstitution.name} />
                    <span>{selectedInstitution.name}</span>
                  </div>
                )
              )}

              <label htmlFor="account-name">
                {t('settings.institutions.modal.name')}
              </label>
              <input
                data-autofocus
                id="account-name"
                type="text"
                placeholder={t('settings.institutions.modal.namePlaceholder')}
                value={name}
                onChange={(e) => {
                  setError('');
                  setName(e.target.value);
                }}
              />

              <label htmlFor="account-type">
                {t('settings.institutions.modal.type')}
              </label>
              <Select
                id="account-type"
                value={type}
                onChange={(e) => setType(e.target.value as AccountType)}
              >
                {ACCOUNT_TYPES.map((value) => (
                  <option key={value} value={value}>
                    {t(`settings.institutions.types.${value}`)}
                  </option>
                ))}
              </Select>

              <div className="account-modal-row">
                <div className="account-modal-field">
                  <label htmlFor="account-last-four">
                    {t('settings.institutions.modal.lastFour')}
                  </label>
                  <input
                    id="account-last-four"
                    type="text"
                    maxLength={4}
                    placeholder="1234"
                    value={lastFour}
                    onChange={(e) => {
                      setError('');
                      setLastFour(e.target.value);
                    }}
                  />
                </div>
                <div className="account-modal-field">
                  <label htmlFor="account-currency">
                    {t('settings.institutions.modal.currency')}
                  </label>
                  <Select
                    id="account-currency"
                    value={currency}
                    onChange={(e) => setCurrency(e.target.value)}
                  >
                    {/* Keeps an account's existing currency selectable even if it isn't one
                        of the usual two. */}
                    {[...new Set([...CURRENCIES, currency])].map((code) => (
                      <option key={code} value={code}>
                        {code}
                      </option>
                    ))}
                  </Select>
                </div>
              </div>

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
                onClick={handleSaveAccount}
                disabled={isPending || institutionId === null}
              >
                {account
                  ? t('settings.institutions.modal.save')
                  : t('settings.institutions.modal.add')}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
};
