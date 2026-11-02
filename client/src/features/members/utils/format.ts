import { apiUrl } from '../../../lib/api-client';
import type { Member } from './types';

/** Name to show for a member: their full name, or their email if they never gave one (members who
 * joined before onboarding asked for a name, until they're next sent back to finish it). */
export const memberDisplayName = (member: Member) =>
  [member.first_name, member.last_name].filter(Boolean).join(' ') ||
  member.email;

/** The `src` for a member's or user's `picture_url`, which the API gives relative to itself. */
export const pictureSrc = (pictureUrl: string | null) =>
  pictureUrl && apiUrl(pictureUrl);
