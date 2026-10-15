/** A row of `GET /members`: a user in the household, with their role. `id` is the
 * user's id. */
export interface Member {
  id: number;
  household_id: number;
  role: 'family_manager' | 'family_member';
  email: string;
  first_name: string | null;
  last_name: string | null;
  status: 'verified' | 'pending' | 'suspended';
  created_at: string;
}
