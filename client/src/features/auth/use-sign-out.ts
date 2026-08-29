import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { logout } from './api';
import { signOutFailedToast } from '../../lib/toast';

/** Ends the session server-side, drops every cached query, and lands back on the sign-in page. */
export const useSignOut = () => {
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  return useMutation({
    mutationFn: logout,
    onSuccess: () => {
      queryClient.clear();
      navigate({ to: '/' });
    },
    onError: signOutFailedToast,
  });
};
