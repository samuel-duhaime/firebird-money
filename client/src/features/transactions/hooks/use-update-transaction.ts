import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateTransaction } from '../utils/api';
import type { TransactionPatch } from '../utils/api';

export const useUpdateTransaction = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: TransactionPatch }) =>
      updateTransaction(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      // A merchant's transaction_count/recommended_category_id are computed from the household's
      // transactions, so changing one (especially its merchant_id or category_id) can change
      // either for the old and/or new merchant.
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
    },
  });
};
