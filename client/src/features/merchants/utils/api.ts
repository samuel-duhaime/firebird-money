import { apiFetch } from '../../../lib/api-client';
import type { Merchant, MerchantSortOrder } from './types';

export const fetchMerchants = (order?: MerchantSortOrder): Promise<Merchant[]> =>
  apiFetch<Merchant[]>(order ? `/merchants?order=${order}` : '/merchants');

export interface NewMerchant {
  name: string;
}

export const createMerchant = (newMerchant: NewMerchant): Promise<Merchant> =>
  apiFetch<Merchant>('/merchants', {
    method: 'POST',
    body: JSON.stringify(newMerchant),
  });

/** Body for `PATCH /merchants/{id}`. Unset fields are left unchanged. Only ever takes effect on a
 * merchant this household owns — a common merchant 404s, same as an unknown id. */
export interface MerchantPatch {
  name?: string;
}

export const updateMerchant = (id: number, patch: MerchantPatch): Promise<Merchant> =>
  apiFetch<Merchant>(`/merchants/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const deleteMerchant = (id: number): Promise<void> =>
  apiFetch<void>(`/merchants/${id}`, { method: 'DELETE' });
