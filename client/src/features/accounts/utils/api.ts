import { apiFetch } from '../../../lib/api-client';
import type { Account, AccountType } from './types';

export const fetchAccounts = (): Promise<Account[]> =>
  apiFetch<Account[]>('/accounts');

export interface NewAccount {
  institution_id: number;
  name: string;
  type: AccountType;
  last_four?: string;
  /** Defaults to `CAD` server-side when absent. */
  currency?: string;
}

export const createAccount = (newAccount: NewAccount): Promise<Account> =>
  apiFetch<Account>('/accounts', {
    method: 'POST',
    body: JSON.stringify(newAccount),
  });

/** Body for `PATCH /accounts/{id}`. Unset fields are left unchanged; `last_four: ''` clears it. */
export type AccountPatch = Partial<NewAccount>;

export const updateAccount = (
  id: number,
  patch: AccountPatch,
): Promise<Account> =>
  apiFetch<Account>(`/accounts/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const deleteAccount = (id: number): Promise<void> =>
  apiFetch<void>(`/accounts/${id}`, { method: 'DELETE' });
