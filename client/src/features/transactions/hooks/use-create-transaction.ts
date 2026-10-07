import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createTransaction } from '../utils/api';
import { addTransactionSucceededToast } from '../../../lib/toast';

export const useCreateTransaction = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createTransaction,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      // A merchant's transaction_count/recommended_category_id are computed from the household's
      // transactions, so a new one can change either for whichever merchant it names.
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
      addTransactionSucceededToast();
    },
  });
};
