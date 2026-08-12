export interface AuthUser {
  id: number;
  email: string;
  status: 'verified' | 'pending' | 'suspended';
  first_name: string | null;
  last_name: string | null;
  avatar_url: string | null;
  created_at: string;
}

/** One household the signed-in user belongs to, with the role they hold in it. */
export interface Membership {
  household_id: number;
  join_code: string;
  type: 'family_manager' | 'family_member';
}

/** Payload of `GET /auth/me`, and of anything that signs the user in. */
export interface CurrentUser {
  user: AuthUser;
  households: Membership[];
}

/**
 * Response to `POST /auth/request-login`. `email_sent` means check your inbox; `signed_in` is the
 * server's `SKIP_EMAIL_VERIFICATION` shortcut, which logs you in without any email at all.
 */
export interface RequestLoginResponse {
  status: 'email_sent' | 'signed_in';
  session?: CurrentUser;
}
