import { apiFetch } from '../../../lib/api-client';
import type { Tag } from './types';

export interface NewTag {
  name: string;
  color: string;
}

export const createTag = (newTag: NewTag): Promise<Tag> =>
  apiFetch<Tag>('/tags', {
    method: 'POST',
    body: JSON.stringify(newTag),
  });

/** Body for `PATCH /tags/{id}`. Unset fields are left unchanged. */
export interface TagPatch {
  name?: string;
  color?: string;
}

export const updateTag = (id: number, patch: TagPatch): Promise<Tag> =>
  apiFetch<Tag>(`/tags/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const deleteTag = (id: number): Promise<void> =>
  apiFetch<void>(`/tags/${id}`, { method: 'DELETE' });
