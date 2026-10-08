/** A row of `GET /household-members`: a membership plus who the member is. */
export interface HouseholdMember {
  id: number;
  household_id: number;
  user_id: number;
  type: 'family_manager' | 'family_member';
  created_at: string;
  email: string;
  first_name: string | null;
  last_name: string | null;
  status: 'verified' | 'pending' | 'suspended';
}
