import type { QueryClient } from '@tanstack/react-query';
import { redirect } from '@tanstack/react-router';
import { ApiError } from '../../../lib/api-client';
import { fetchCurrentUser } from './api';
import { currentUserQueryKey } from '../hooks/use-current-user';
import type { CurrentUser } from './types';

const isUnauthorized = (error: unknown) =>
  error instanceof ApiError && error.status === 401;

/** Whether the user has finished every onboarding step. */
export const isOnboarded = (session: CurrentUser) =>
  session.pending_onboarding_steps.length === 0;

/** Where a signed-in user belongs: onboarding until they've finished it, the dashboard after. */
export const homePathFor = (session: CurrentUser) =>
  isOnboarded(session) ? '/dashboard' : '/onboarding';

/**
 * Route `beforeLoad` guard: redirects to `/sign-in` when there's no live session (`GET /auth/me`
 * answers 401), otherwise returns the session. Always hits the network — a route guard can't
 * accept a stale cached session — so a revoked or expired session is caught even when
 * `useCurrentUser` still has fresh-looking data cached from before it died.
 */
export const requireAuth = async (
  queryClient: QueryClient,
): Promise<CurrentUser> => {
  try {
    return await queryClient.fetchQuery({
      queryKey: currentUserQueryKey,
      queryFn: fetchCurrentUser,
      staleTime: 0,
      retry: false,
    });
  } catch (error) {
    if (!isUnauthorized(error)) throw error;
    throw redirect({ to: '/sign-in' });
  }
};

/**
 * Route `beforeLoad` guard for the app itself: like `requireAuth`, but also sends anyone who hasn't
 * finished onboarding back to `/onboarding` — whichever page they tried to open.
 */
export const requireOnboarded = async (queryClient: QueryClient) => {
  const session = await requireAuth(queryClient);
  if (!isOnboarded(session)) {
    throw redirect({ to: '/onboarding' });
  }
};

/** Route `beforeLoad` guard for `/onboarding`: only reachable until onboarding is finished. */
export const requireNotOnboarded = async (queryClient: QueryClient) => {
  const session = await requireAuth(queryClient);
  if (isOnboarded(session)) {
    throw redirect({ to: '/dashboard' });
  }
};

/**
 * Route `beforeLoad` guard for pages that only make sense signed out (sign-in): sends an
 * already-signed-in visitor on to the same place `SignInPage` sends a fresh sign-in.
 */
export const redirectIfAuthenticated = async (queryClient: QueryClient) => {
  let session;
  try {
    session = await queryClient.fetchQuery({
      queryKey: currentUserQueryKey,
      queryFn: fetchCurrentUser,
      staleTime: 0,
      retry: false,
    });
  } catch (error) {
    if (!isUnauthorized(error)) throw error;
    return;
  }
  throw redirect({ to: homePathFor(session) });
};
