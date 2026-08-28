import type { QueryClient } from '@tanstack/react-query';
import { redirect } from '@tanstack/react-router';
import { fetchCurrentUser } from './api';
import { currentUserQueryKey } from './use-current-user';

/**
 * Route `beforeLoad` guard: redirects to `/sign-in` when there's no live session (`GET /auth/me`
 * answers 401), otherwise lets the navigation through. Reuses whatever `useCurrentUser` already
 * has cached, so this doesn't cause an extra fetch on top of what the page renders.
 */
export const requireAuth = async (queryClient: QueryClient) => {
  try {
    await queryClient.ensureQueryData({
      queryKey: currentUserQueryKey,
      queryFn: fetchCurrentUser,
      retry: false,
    });
  } catch {
    throw redirect({ to: '/sign-in' });
  }
};
