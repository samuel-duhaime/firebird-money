import { useMutation, useQueryClient } from '@tanstack/react-query';
import { bulkUpdateTransactions } from '../utils/api';
import type { BulkTransactionPatch } from '../utils/api';

export const useBulkUpdateTransactions = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ ids, patch }: { ids: number[]; patch: BulkTransactionPatch }) =>
      bulkUpdateTransactions(ids, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
