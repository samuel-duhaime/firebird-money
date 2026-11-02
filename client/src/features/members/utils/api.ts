import { apiFetch, apiFetchUpload } from '../../../lib/api-client';
import type { Member } from './types';

/** Sets or replaces a member's picture and returns the updated member. */
export const uploadMemberPicture = (id: number, picture: Blob) => {
  const formData = new FormData();
  formData.append('file', picture, 'picture.jpg');
  return apiFetchUpload<Member>(`/members/${id}/picture`, formData, 'PUT');
};

/** Removes a member's picture and returns the updated member. */
export const deleteMemberPicture = (id: number) =>
  apiFetch<Member>(`/members/${id}/picture`, { method: 'DELETE' });
