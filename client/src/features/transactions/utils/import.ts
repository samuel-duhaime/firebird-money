import { apiFetch } from '../../../lib/api-client';
import type { AccountType } from '../../accounts/utils/types';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL as string;

/** Largest statement the server accepts (its multipart field limit). */
export const MAX_STATEMENT_BYTES = 10 * 1024 * 1024;

/** What the file picker offers — the server decides from the file's own bytes, not these. */
export const STATEMENT_FILE_ACCEPT =
  '.pdf,.png,.jpg,.jpeg,.gif,.webp,.csv,.tsv,.txt,.xlsx,.xls,.ods,application/pdf,image/*,text/csv';

/** One statement line read by the server, already matched to the household. Nothing is stored
 * until the user accepts it. */
export interface StatementTransaction {
  date: string;
  /** The line's description exactly as the statement printed it. */
  original_statement: string;
  /** Always positive; direction comes from the category. */
  amount: string;
  /** Exactly one of `merchant_id` and `new_merchant_name` is set. */
  merchant_id: number | null;
  /** A merchant the household doesn't have yet, created only if the line is accepted. */
  new_merchant_name: string | null;
  category_id: number | null;
  /** `false` when the category is a guess the user should double-check. */
  category_confident: boolean;
  /** `null` when no account of the household matched — see `new_account`. */
  account_id: number | null;
  /** The account the statement names when the household doesn't have it yet; created only if a
   * line using it is accepted. Only set when `account_id` is `null`. */
  new_account: SuggestedAccount | null;
  tag_ids: number[];
  /** An existing transaction with the same date, amount, and account — likely this same line. */
  duplicate_of_transaction_id: number | null;
}

/** An account a statement names that the household doesn't have yet. */
export interface SuggestedAccount {
  institution_name: string;
  name: string;
  type: AccountType;
  last_four: string | null;
}

/** One line of `POST /transactions/import/read`'s streamed (newline-delimited JSON) response. */
export type StatementReadEvent =
  | { type: 'reading' }
  | { type: 'found'; count: number }
  | { type: 'matching' }
  | { type: 'checking_duplicates' }
  | { type: 'done'; transactions: StatementTransaction[] }
  | { type: 'error'; error: string };

/** Thrown by `readStatement` with the server's own (localized) message, when it gave one. */
export class StatementReadError extends Error {
  status?: number;

  constructor(message: string, status?: number) {
    super(message);
    this.name = 'StatementReadError';
    this.status = status;
  }
}

/** Splits a stream of bytes into lines, calling `onLine` with each complete one. */
const readLines = async (
  body: ReadableStream<Uint8Array>,
  onLine: (line: string) => void,
) => {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() ?? '';
    lines.filter((line) => line.trim() !== '').forEach(onLine);
  }
  if (buffer.trim() !== '') onLine(buffer);
};

/**
 * Sends a statement to be read and calls `onEvent` with each progress event as it arrives.
 * Resolves with the lines once the `done` event comes; rejects with a `StatementReadError` on an
 * `error` event or an error response. Aborting `signal` stops the read on the server too.
 */
export const readStatement = async (
  file: File,
  accountId: number | null,
  onEvent: (event: StatementReadEvent) => void,
  signal: AbortSignal,
): Promise<StatementTransaction[]> => {
  const formData = new FormData();
  if (accountId !== null) formData.set('account_id', String(accountId));
  formData.set('file', file);

  const response = await fetch(`${API_BASE_URL}/transactions/import/read`, {
    // Sends the session cookie: the API is a different origin, so the browser withholds it
    // otherwise.
    credentials: 'include',
    method: 'POST',
    body: formData,
    signal,
  });

  if (!response.ok || !response.body) {
    const body = (await response.json().catch(() => null)) as {
      error?: string;
    } | null;
    throw new StatementReadError(body?.error ?? '', response.status);
  }

  let result: StatementTransaction[] | undefined;
  let failure: string | undefined;
  await readLines(response.body, (line) => {
    const event = JSON.parse(line) as StatementReadEvent;
    if (event.type === 'done') result = event.transactions;
    if (event.type === 'error') failure = event.error;
    onEvent(event);
  });

  if (result) return result;
  throw new StatementReadError(failure ?? '');
};

/** One accepted line for `POST /transactions/import`. */
export interface ImportTransaction {
  date: string;
  original_statement: string;
  amount: string;
  merchant_id?: number;
  new_merchant_name?: string;
  category_id: number;
  /** Exactly one of `account_id` and `new_account` is set. */
  account_id?: number;
  new_account?: SuggestedAccount;
  tag_ids: number[];
}

export interface ImportResult {
  created_count: number;
  created_merchant_count: number;
  created_account_count: number;
}

/** Adds the accepted lines, all or nothing, each marked as imported. */
export const importTransactions = (
  transactions: ImportTransaction[],
): Promise<ImportResult> =>
  apiFetch<ImportResult>('/transactions/import', {
    method: 'POST',
    body: JSON.stringify({ transactions }),
  });
