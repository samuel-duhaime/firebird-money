import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteAccount } from '../utils/api';

export const useDeleteAccount = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteAccount,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      queryClient.invalidateQueries({ queryKey: ['institutions'] });
    },
  });
};
