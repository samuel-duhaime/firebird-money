import { apiFetch } from '../../lib/api-client';
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
