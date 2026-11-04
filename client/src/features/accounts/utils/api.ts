import { apiFetch } from '../../../lib/api-client';
import type { Account, AccountType } from './types';

export const fetchAccounts = (): Promise<Account[]> =>
  apiFetch<Account[]>('/accounts');

/** Who something belongs to, as the API takes it: `'shared'` or a member's id. */
export type Owner = 'shared' | number;

/** The API's `Owner` for an `owner_user_id` (`null` meaning Shared). */
export const toOwner = (ownerUserId: number | null): Owner =>
  ownerUserId ?? 'shared';

export interface NewAccount {
  institution_id: number;
  name: string;
  type: AccountType;
  last_four?: string;
  /** Defaults to `CAD` server-side when absent. */
  currency?: string;
  /** `'shared'` or a member's id. Required: there's no default owner. */
  owner: Owner;
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

/** Body for `PATCH /accounts/owners`: each account's new owner, all saved or none. */
export interface AccountOwner {
  account_id: number;
  owner: Owner;
}

/** Answers with every account. Their transactions follow, as on `updateAccount`. */
export const updateAccountOwners = (
  owners: AccountOwner[],
): Promise<Account[]> =>
  apiFetch<Account[]>('/accounts/owners', {
    method: 'PATCH',
    body: JSON.stringify({ owners }),
  });
