import { useQuery } from '@tanstack/react-query';
import { getImportJob } from './import';

const isTerminal = (status?: string) =>
  status === 'succeeded' || status === 'failed';

/** Polls an import job's status until it reaches a terminal state (succeeded/failed). */
export const useImportJob = (jobId?: string) =>
  useQuery({
    queryKey: ['import-job', jobId ?? null],
    queryFn: () => getImportJob(jobId!),
    enabled: jobId !== undefined,
    refetchInterval: (query) =>
      isTerminal(query.state.data?.status) ? false : 2000,
  });
