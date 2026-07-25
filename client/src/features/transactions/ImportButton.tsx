import { useEffect, useRef, useState } from 'react';
import type { ChangeEvent } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { faFileImport, faSpinner } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useImportJob } from './use-import-job';
import { startImport } from './import';
import {
  importFailedToast,
  importStartedToast,
  importSucceededToast,
} from '../../lib/toast';

export const ImportButton = () => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [jobId, setJobId] = useState<string>();
  const queryClient = useQueryClient();

  const { data: job } = useImportJob(jobId);

  useEffect(() => {
    if (!job) return;
    if (job.status === 'succeeded') {
      importSucceededToast(job.created_count ?? 0);
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
    if (!file) return;

    try {
      const startedJob = await startImport(file);
      setJobId(startedJob.id);
      importStartedToast();
    } catch {
      importFailedToast();
    }
  };

  return (
    <>
      <input ref={inputRef} type="file" hidden onChange={handleFileChange} />
      <TopMenuButton
        icon={jobId ? faSpinner : faFileImport}
        spin={jobId !== undefined}
        label={
          jobId
            ? job?.status === 'pending'
              ? 'Uploading…'
              : 'Importing…'
            : 'Import'
        }
        onClick={() => inputRef.current?.click()}
      />
    </>
  );
};
