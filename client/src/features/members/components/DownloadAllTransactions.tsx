import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCheck, faDownload } from '@fortawesome/free-solid-svg-icons';
import { downloadFailedToast } from '../../../lib/toast';
import { downloadTransactions } from '../../transactions/utils/download';
import type { DownloadFormat } from '../../transactions/utils/download';
import './DownloadAllTransactions.css';

type DownloadAllTransactionsProps = {
  /** From `useHouseholdTransactionCount`: `null` while loading. */
  count: number | null;
  /** Called once a file has been downloaded. */
  onDownloaded: () => void;
};

/**
 * Every transaction in the household as one CSV or Excel file, before leaving or deleting it, so
 * they can be imported again from the Transactions page in another household.
 */
export const DownloadAllTransactions = ({
  count,
  onDownloaded,
}: DownloadAllTransactionsProps) => {
  const { t } = useTranslation();
  const [downloading, setDownloading] = useState(false);
  const [downloaded, setDownloaded] = useState(false);

  if (count === 0) {
    return (
      <p className="download-all-empty">
        {t('settings.members.household.nothingToDownload')}
      </p>
    );
  }

  const download = async (format: DownloadFormat) => {
    setDownloading(true);
    try {
      await downloadTransactions(format, {});
      setDownloaded(true);
      onDownloaded();
    } catch {
      downloadFailedToast();
    } finally {
      setDownloading(false);
    }
  };

  return (
    <section className="download-all">
      <h3>{t('settings.members.household.downloadStep')}</h3>
      <p>
        {t('settings.members.household.downloadDescription', {
          count: count ?? 0,
        })}
      </p>
      <div className="download-all-actions">
        {(['csv', 'xlsx'] as const).map((format) => (
          <button
            key={format}
            type="button"
            className="modal-button modal-button--primary"
            onClick={() => download(format)}
            disabled={count === null || downloading}
          >
            <FontAwesomeIcon icon={faDownload} />
            {format === 'csv' ? 'CSV' : 'Excel'}
          </button>
        ))}
        {downloaded && (
          <span className="download-all-done">
            <FontAwesomeIcon icon={faCheck} />
            {t('settings.members.household.downloaded')}
          </span>
        )}
      </div>
    </section>
  );
};
