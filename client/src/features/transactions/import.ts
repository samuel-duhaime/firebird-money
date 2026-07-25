import { apiFetch, apiFetchUpload } from '../../lib/api-client';
import type { ImportJob } from './types';

/** Uploads a budget file for the server to import in the background; returns the created job. */
export const startImport = async (file: File): Promise<ImportJob> => {
  const formData = new FormData();
  formData.set('file', file);
  return apiFetchUpload<ImportJob>('/transactions/import', formData);
};

export const getImportJob = (id: string): Promise<ImportJob> =>
  apiFetch<ImportJob>(`/transactions/import/jobs/${id}`);
