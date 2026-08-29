import type { QueryClient } from '@tanstack/react-query';
import { redirect } from '@tanstack/react-router';
import { ApiError } from '../../lib/api-client';
import { fetchCurrentUser } from './api';
import { currentUserQueryKey } from './use-current-user';

const isUnauthorized = (error: unknown) => error instanceof ApiError && error.status === 401;

/**
 * Route `beforeLoad` guard: redirects to `/sign-in` when there's no live session (`GET /auth/me`
 * answers 401), otherwise lets the navigation through. Always hits the network — a route guard
 * can't accept a stale cached session — so a revoked or expired session is caught even when
 * `useCurrentUser` still has fresh-looking data cached from before it died.
 */
export const requireAuth = async (queryClient: QueryClient) => {
  try {
    await queryClient.fetchQuery({
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
 * Route `beforeLoad` guard for pages that only make sense signed out (sign-in): sends an
 * already-signed-in visitor on to the same place `SignInPage` sends a fresh sign-in.
 */
export const redirectIfAuthenticated = async (queryClient: QueryClient) => {
  let user;
  try {
    user = await queryClient.fetchQuery({
      queryKey: currentUserQueryKey,
      queryFn: fetchCurrentUser,
      staleTime: 0,
      retry: false,
    });
  } catch (error) {
    if (!isUnauthorized(error)) throw error;
    return;
  }
  throw redirect({ to: user.households.length > 0 ? '/dashboard' : '/onboarding' });
};
