import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateAccount } from '../utils/api';
import type { AccountPatch } from '../utils/api';

export const useUpdateAccount = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: AccountPatch }) =>
      updateAccount(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      queryClient.invalidateQueries({ queryKey: ['institutions'] });
      // A transaction's joined `account_name` goes stale after a rename here.
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
