import { apiFetchFile } from '../../lib/api-client';
import { downloadBlob } from '../../lib/download-file';
import type { SortOrder } from './types';

export type DownloadFormat = 'csv' | 'xlsx';

/** Downloads the same (filtered/sorted) transactions shown on the page, as a CSV or Excel file. */
export const downloadTransactions = async (
  format: DownloadFormat,
  search?: string,
  order?: SortOrder,
) => {
  const params = new URLSearchParams({ format });
  if (search) params.set('search', search);
  if (order) params.set('order', order);

  const { blob, filename } = await apiFetchFile(
    `/transactions/download?${params.toString()}`,
  );
  downloadBlob(blob, filename ?? `transactions.${format}`);
};
