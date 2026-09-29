import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteTransaction } from '../utils/api';

export const useDeleteTransaction = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteTransaction,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
