import { useEffect, useId, useState } from 'react';
import type { ChangeEvent, DragEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import type { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import {
  faCircleCheck,
  faCircleExclamation,
  faEyeSlash,
  faShieldHalved,
  faTrashCan,
  faUpload,
} from '@fortawesome/free-solid-svg-icons';
import { Select } from '../../../../components/Select';
import { useAccounts } from '../../../accounts/hooks/use-accounts';
import { accountLabel } from '../../../accounts/utils/format';
import { STATEMENT_FILE_ACCEPT } from '../../utils/import';

type StatementUploadProps = {
  /** `null` = let the statement say which account it is. */
  accountId: number | null;
  onAccountChange: (accountId: number | null) => void;
  onFile: (file: File) => void;
  /** Why the last attempt failed, shown above the drop zone. */
  error?: string;
};

const PrivacyPoint = ({
  icon,
  title,
  text,
}: {
  icon: IconDefinition;
  title: string;
  text: string;
}) => (
  <li className="statement-privacy-point">
    <span className="statement-privacy-point-icon" aria-hidden="true">
      <FontAwesomeIcon icon={icon} />
    </span>
    <strong className="statement-privacy-point-title">{title}</strong>
    <span className="statement-privacy-point-text">{text}</span>
  </li>
);

/** The first step: pick (drop, choose, or paste) a statement and, optionally, its account. */
export const StatementUpload = ({
  accountId,
  onAccountChange,
  onFile,
  error,
}: StatementUploadProps) => {
  const { t } = useTranslation();
  const { data: accounts } = useAccounts();
  const [isDragging, setIsDragging] = useState(false);
  const fileInputId = useId();
  const accountSelectId = useId();

  // A screenshot copied to the clipboard can be pasted anywhere on the page.
  useEffect(() => {
    const handlePaste = (event: ClipboardEvent) => {
      const file = event.clipboardData?.files[0];
      if (file) {
        event.preventDefault();
        onFile(file);
      }
    };
    window.addEventListener('paste', handlePaste);
    return () => window.removeEventListener('paste', handlePaste);
  }, [onFile]);

  const handleDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setIsDragging(false);
    const file = event.dataTransfer.files[0];
    if (file) onFile(file);
  };

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (file) onFile(file);
  };

  return (
    <div className="statement-import">
      <p className="statement-import-intro">{t('transactions.import.intro')}</p>

      {error && (
        <p className="statement-import-error" role="alert">
          <FontAwesomeIcon icon={faCircleExclamation} aria-hidden="true" />
          {error}
        </p>
      )}

      <div
        className={
          isDragging
            ? 'statement-drop-zone statement-drop-zone--dragging'
            : 'statement-drop-zone'
        }
        onDragOver={(event) => {
          event.preventDefault();
          setIsDragging(true);
        }}
        onDragLeave={() => setIsDragging(false)}
        onDrop={handleDrop}
      >
        <span className="statement-drop-zone-icon" aria-hidden="true">
          <FontAwesomeIcon icon={faUpload} />
        </span>
        <p className="statement-drop-zone-title">
          {isDragging
            ? t('transactions.import.dropActive')
            : t('transactions.import.dropTitle')}
        </p>
        <p className="statement-drop-zone-formats">
          {t('transactions.import.formats')}
        </p>
        <input
          id={fileInputId}
          type="file"
          accept={STATEMENT_FILE_ACCEPT}
          className="statement-drop-zone-input"
          onChange={handleChange}
        />
        <label htmlFor={fileInputId} className="statement-button">
          {t('transactions.import.choose')}
        </label>
      </div>

      <div className="statement-account">
        <label htmlFor={accountSelectId} className="statement-account-label">
          {t('transactions.import.account')}
        </label>
        <Select
          id={accountSelectId}
          value={accountId ?? ''}
          onChange={(event) =>
            onAccountChange(
              event.target.value === '' ? null : Number(event.target.value),
            )
          }
        >
          <option value="">{t('transactions.import.detectAccount')}</option>
          {accounts?.map((account) => (
            <option key={account.id} value={account.id}>
              {accountLabel(account)}
            </option>
          ))}
        </Select>
      </div>

      <section
        className="statement-privacy"
        aria-labelledby="statement-privacy-title"
      >
        <div className="statement-privacy-header">
          <span className="statement-privacy-badge" aria-hidden="true">
            <FontAwesomeIcon icon={faShieldHalved} />
          </span>
          <div>
            <h2
              id="statement-privacy-title"
              className="statement-privacy-title"
            >
              {t('transactions.import.privacy.title')}
            </h2>
            <p className="statement-privacy-subtitle">
              {t('transactions.import.privacy.subtitle')}
            </p>
          </div>
        </div>
        <ul className="statement-privacy-points">
          <PrivacyPoint
            icon={faTrashCan}
            title={t('transactions.import.privacy.neverSaved')}
            text={t('transactions.import.privacy.neverSavedText')}
          />
          <PrivacyPoint
            icon={faEyeSlash}
            title={t('transactions.import.privacy.noTraining')}
            text={t('transactions.import.privacy.noTrainingText')}
          />
          <PrivacyPoint
            icon={faCircleCheck}
            title={t('transactions.import.privacy.youDecide')}
            text={t('transactions.import.privacy.youDecideText')}
          />
        </ul>
      </section>
    </div>
  );
};
