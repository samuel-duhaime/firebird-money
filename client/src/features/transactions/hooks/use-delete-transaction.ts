import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteTransaction } from '../utils/api';

export const useDeleteTransaction = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteTransaction,
    onSuccess: (_data, id) => {
      // Removed first: invalidating ['transactions'] alone would also match this exact detail
      // query (still active — the edit panel unmounts after this callback) and refetch it,
      // hitting a 404 for the record just deleted.
      queryClient.removeQueries({ queryKey: ['transactions', id], exact: true });
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
