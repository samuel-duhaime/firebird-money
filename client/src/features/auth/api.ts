import { apiFetch } from '../../lib/api-client';
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

/** Creates a household (no code) or joins an existing one (with a code). */
export const submitOnboarding = (joinCode?: string): Promise<CurrentUser> =>
  apiFetch<CurrentUser>('/auth/onboarding', {
    method: 'POST',
    body: JSON.stringify({ join_code: joinCode ?? null }),
  });
