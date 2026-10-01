import { apiFetch } from '../../../lib/api-client';
import type { Transaction } from './types';

export interface NewTransaction {
  date: string;
  merchant: string;
  amount: string;
  category_id: number;
  account: string;
  /** Optional; absent or `[]` means no tags. */
  tag_ids?: number[];
}

export const createTransaction = (
  newTransaction: NewTransaction,
): Promise<Transaction> =>
  apiFetch<Transaction>('/transactions', {
    method: 'POST',
    body: JSON.stringify(newTransaction),
  });

/** Body for `PATCH /transactions/{id}`. Unset fields are left unchanged. `tag_ids`, if given,
 * *replaces* the full tag set (`[]` clears it) — unlike `BulkTransactionPatch.tag_ids`, which only
 * adds. */
export interface TransactionPatch {
  date?: string;
  merchant?: string;
  amount?: string;
  category_id?: number;
  account?: string;
  tag_ids?: number[];
}

export const updateTransaction = (
  id: number,
  patch: TransactionPatch,
): Promise<Transaction> =>
  apiFetch<Transaction>(`/transactions/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const getTransaction = (id: number): Promise<Transaction> =>
  apiFetch<Transaction>(`/transactions/${id}`);

export const deleteTransaction = (id: number): Promise<void> =>
  apiFetch<void>(`/transactions/${id}`, { method: 'DELETE' });

/** Body for `PATCH /transactions/bulk` — the "edit multiple" panel's field set, a subset of
 * `TransactionPatch`: no `amount`/`account`, since setting one value across several different
 * transactions doesn't make sense for those fields. `tag_ids` here *adds* to each transaction's
 * existing tags rather than replacing them, unlike `TransactionPatch.tag_ids`. */
export type BulkTransactionPatch = Pick<
  TransactionPatch,
  'date' | 'merchant' | 'category_id' | 'tag_ids'
>;

export const bulkUpdateTransactions = (
  ids: number[],
  patch: BulkTransactionPatch,
): Promise<Transaction[]> =>
  apiFetch<Transaction[]>('/transactions/bulk', {
    method: 'PATCH',
    body: JSON.stringify({ ids, patch }),
  });

export const bulkDeleteTransactions = (ids: number[]): Promise<void> =>
  apiFetch<void>('/transactions/bulk', {
    method: 'DELETE',
    body: JSON.stringify({ ids }),
  });
