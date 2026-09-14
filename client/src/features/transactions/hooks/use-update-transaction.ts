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
    },
  });
};
