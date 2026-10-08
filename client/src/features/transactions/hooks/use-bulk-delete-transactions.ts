import { useMutation, useQueryClient } from '@tanstack/react-query';
import { bulkDeleteTransactions } from '../utils/api';

export const useBulkDeleteTransactions = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: bulkDeleteTransactions,
    onSuccess: (_data, ids) => {
      // Removed first, same reasoning as `use-delete-transaction.ts`: invalidating
      // ['transactions'] alone would also match these exact detail queries and refetch them,
      // hitting a 404 for records just deleted.
      for (const id of ids) {
        queryClient.removeQueries({ queryKey: ['transactions', id], exact: true });
      }
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      // See use-update-transaction.ts — a merchant's transaction_count/recommended_category_id
      // are computed from the household's transactions.
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
    },
  });
};
