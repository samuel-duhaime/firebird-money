import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { faFileImport, faSpinner } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useImportJob } from './use-import-job';
import { startImport } from './import';
import {
  importFailedToast,
  importPartialToast,
  importStartedToast,
  importSucceededToast,
} from '../../lib/toast';

export const ImportButton = () => {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [jobId, setJobId] = useState<string>();
  const queryClient = useQueryClient();

  const { data: job } = useImportJob(jobId);
  const busy = isUploading || jobId !== undefined;

  useEffect(() => {
    if (!job) return;
    if (job.status === 'succeeded') {
      const failedCount = job.failed_count ?? 0;
      const skippedCount = job.skipped_count ?? 0;
      if (failedCount > 0 || skippedCount > 0) {
        importPartialToast(job.created_count ?? 0, failedCount, skippedCount);
      } else {
        importSucceededToast(job.created_count ?? 0);
      }
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      setJobId(undefined);
    } else if (job.status === 'failed') {
      importFailedToast();
      setJobId(undefined);
    }
  }, [job, queryClient]);

  const handleFileChange = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file || busy) return;

    setIsUploading(true);
    try {
      const startedJob = await startImport(file);
      setJobId(startedJob.id);
      importStartedToast();
    } catch {
      importFailedToast();
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <>
      <input
        ref={inputRef}
        type="file"
        hidden
        disabled={busy}
        onChange={handleFileChange}
      />
      <TopMenuButton
        icon={busy ? faSpinner : faFileImport}
        spin={busy}
        disabled={busy}
        label={
          busy
            ? t(isUploading ? 'transactions.import.uploading' : 'transactions.import.importing')
            : t('transactions.import.trigger')
        }
        onClick={() => inputRef.current?.click()}
      />
    </>
  );
};
