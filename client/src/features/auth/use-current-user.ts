import { useQuery, useQueryClient } from '@tanstack/react-query';
import { fetchCurrentUser } from './api';
import type { CurrentUser } from './types';

export const currentUserQueryKey = ['auth', 'me'] as const;

/**
 * The signed-in user, or an error when there's no live session (`GET /auth/me` answers 401).
 *
 * Not retried: a 401 is a settled answer, and retrying it just delays showing the sign-in page.
 */
export const useCurrentUser = () =>
  useQuery({
    queryKey: currentUserQueryKey,
    queryFn: fetchCurrentUser,
    retry: false,
  });

/** Seeds the cache after a sign-in, so the app doesn't refetch what the response already held. */
export const useSetCurrentUser = () => {
  const queryClient = useQueryClient();

  return (session: CurrentUser | null) => {
    queryClient.setQueryData(currentUserQueryKey, session ?? undefined);
    if (session === null) {
      queryClient.clear();
    }
  };
};
