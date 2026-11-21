import i18n from '../../../i18n';
import { apiFetchFile } from '../../../lib/api-client';
import { downloadBlob } from '../../../lib/download-file';
import { transactionsQueryParams } from './filters';
import type { TransactionsQuery } from './filters';

export type DownloadFormat = 'csv' | 'xlsx';

/** Downloads the same (filtered/sorted) transactions shown on the page, as a CSV or Excel file
 * written in the app's current language (headers, category names, filename). */
export const downloadTransactions = async (
  format: DownloadFormat,
  query: TransactionsQuery,
) => {
  const params = transactionsQueryParams(query);
  params.set('format', format);
  params.set('language', i18n.resolvedLanguage ?? i18n.language);

  const { blob, filename } = await apiFetchFile(
    `/transactions/download?${params.toString()}`,
  );
  downloadBlob(blob, filename ?? `transactions.${format}`);
};
