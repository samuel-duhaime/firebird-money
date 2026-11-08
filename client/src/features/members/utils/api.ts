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

/** Changes a member's role and returns the updated member. Managers only. */
export const updateMemberRole = (id: number, role: Member['role']) =>
  apiFetch<Member>(`/members/${id}`, {
    method: 'PATCH',
    body: JSON.stringify({ role }),
  });

/** Removes someone else from the household. Managers only. */
export const removeMember = (id: number) =>
  apiFetch<void>(`/members/${id}`, { method: 'DELETE' });

/** Gives the household a new invite code, so the old one stops working. Managers only. */
export const regenerateJoinCode = (householdId: number) =>
  apiFetch<{ id: number; join_code: string }>(
    `/households/${householdId}/join-code`,
    { method: 'POST' },
  );

/** Takes the signed-in user out of their household. What they added stays in it. */
export const leaveHousehold = (householdId: number) =>
  apiFetch<void>(`/households/${householdId}/leave`, { method: 'POST' });

/** Deletes the household and everything in it. Only for its manager, when they're alone in it. */
export const deleteHousehold = (householdId: number) =>
  apiFetch<void>(`/households/${householdId}`, { method: 'DELETE' });
