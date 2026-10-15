import { apiFetchFile } from '../../../lib/api-client';
import { downloadBlob } from '../../../lib/download-file';
import { transactionsQueryParams } from './filters';
import type { TransactionsQuery } from './filters';

export type DownloadFormat = 'csv' | 'xlsx';

/** Downloads the same (filtered/sorted) transactions shown on the page, as a CSV or Excel file. */
export const downloadTransactions = async (
  format: DownloadFormat,
  query: TransactionsQuery,
) => {
  const params = transactionsQueryParams(query);
  params.set('format', format);

  const { blob, filename } = await apiFetchFile(
    `/transactions/download?${params.toString()}`,
  );
  downloadBlob(blob, filename ?? `transactions.${format}`);
};
