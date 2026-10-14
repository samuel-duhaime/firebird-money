import { apiFetch } from '../../../lib/api-client';
import type { Institution } from './types';

export const fetchInstitutions = (): Promise<Institution[]> =>
  apiFetch<Institution[]>('/institutions');

export interface NewInstitution {
  name: string;
  website_url?: string;
}

export const createInstitution = (
  newInstitution: NewInstitution,
): Promise<Institution> =>
  apiFetch<Institution>('/institutions', {
    method: 'POST',
    body: JSON.stringify(newInstitution),
  });

/** Body for `PATCH /institutions/{id}`. Unset fields are left unchanged; `website_url: ''` clears
 * it. Only ever takes effect on a custom institution — a common one 404s. */
export interface InstitutionPatch {
  name?: string;
  website_url?: string;
}

export const updateInstitution = (
  id: number,
  patch: InstitutionPatch,
): Promise<Institution> =>
  apiFetch<Institution>(`/institutions/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const deleteInstitution = (id: number): Promise<void> =>
  apiFetch<void>(`/institutions/${id}`, { method: 'DELETE' });
