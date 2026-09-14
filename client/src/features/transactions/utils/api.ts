import { apiFetch } from '../../../lib/api-client';
import type { Transaction } from './types';

export interface NewTransaction {
  date: string;
  merchant: string;
  amount: string;
  category_id: number;
  account: string;
}

export const createTransaction = (
  newTransaction: NewTransaction,
): Promise<Transaction> =>
  apiFetch<Transaction>('/transactions', {
    method: 'POST',
    body: JSON.stringify(newTransaction),
  });

/** Body for `PATCH /transactions/{id}`. Unset fields are left unchanged. */
export interface TransactionPatch {
  date?: string;
  merchant?: string;
  amount?: string;
  category_id?: number;
  account?: string;
}

export const updateTransaction = (
  id: number,
  patch: TransactionPatch,
): Promise<Transaction> =>
  apiFetch<Transaction>(`/transactions/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });
