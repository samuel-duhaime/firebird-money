import { useMutation, useQueryClient } from '@tanstack/react-query';
import { importTransactions } from '../utils/import';
import { importSucceededToast } from '../../../lib/toast';

export const useImportTransactions = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: importTransactions,
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      // New merchants may have been created, and every merchant's transaction_count can change.
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
      // A statement can name an account (and bank) the household didn't have yet.
      if (result.created_account_count > 0) {
        queryClient.invalidateQueries({ queryKey: ['accounts'] });
        queryClient.invalidateQueries({ queryKey: ['institutions'] });
      }
      importSucceededToast(
        result.created_count,
        result.created_merchant_count,
        result.created_account_count,
      );
    },
  });
};
