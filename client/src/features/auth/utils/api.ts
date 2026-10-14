import { apiFetch } from '../../../lib/api-client';
import type { CurrentUser, RequestLoginResponse } from './types';

/**
 * Asks for a magic link. Creates the account if this email has never signed in before.
 *
 * `language` is the UI's current language, so the email arrives in the one the person was just
 * reading rather than the server's default.
 */
export const requestLogin = (
  email: string,
  language: string,
): Promise<RequestLoginResponse> =>
  apiFetch<RequestLoginResponse>('/auth/request-login', {
    method: 'POST',
    body: JSON.stringify({ email, language }),
  });

/** Spends the token from a magic link and opens a session. */
export const verifyLogin = (token: string): Promise<CurrentUser> =>
  apiFetch<CurrentUser>(`/auth/verify?token=${encodeURIComponent(token)}`);

export const fetchCurrentUser = (): Promise<CurrentUser> =>
  apiFetch<CurrentUser>('/auth/me');

export const logout = (): Promise<void> =>
  apiFetch<void>('/auth/logout', { method: 'POST' });

export interface OnboardingRequest {
  firstName: string;
  lastName: string;
  /** Joins that household; without one, a new household is created (or, if the user already
   * belongs to one, only the name is saved). */
  joinCode?: string;
}

/** Saves the user's name, settles their household, and marks onboarding as finished. */
export const submitOnboarding = ({
  firstName,
  lastName,
  joinCode,
}: OnboardingRequest): Promise<CurrentUser> =>
  apiFetch<CurrentUser>('/auth/onboarding', {
    method: 'POST',
    body: JSON.stringify({
      first_name: firstName,
      last_name: lastName === '' ? null : lastName,
      join_code: joinCode ?? null,
    }),
  });
