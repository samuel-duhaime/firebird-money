import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createAccount } from '../utils/api';

export const useCreateAccount = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createAccount,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      // Each institution carries its `account_count`.
      queryClient.invalidateQueries({ queryKey: ['institutions'] });
    },
  });
};
