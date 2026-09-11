export interface AuthUser {
  id: number;
  email: string;
  status: 'verified' | 'pending' | 'suspended';
  first_name: string | null;
  last_name: string | null;
  avatar_url: string | null;
  created_at: string;
}

/** The household a user belongs to (at most one, ever), with the role they hold in it. `id` is
 * the `household_members` row's own id. */
export interface Membership {
  id: number;
  household_id: number;
  join_code: string;
  type: 'family_manager' | 'family_member';
}

/** Payload of `GET /auth/me`, and of anything that signs the user in. `household` is `null`
 * before onboarding. */
export interface CurrentUser {
  user: AuthUser;
  household: Membership | null;
}

/**
 * Response to `POST /auth/request-login`. `email_sent` means check your inbox; `signed_in` is the
 * server's `SKIP_EMAIL_VERIFICATION` shortcut, which logs you in without any email at all — a
 * discriminated union so `session` is only reachable on the branch that actually has one.
 */
export type RequestLoginResponse =
  { status: 'email_sent' } | { status: 'signed_in'; session: CurrentUser };
