import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createTransaction } from './api';
import { addTransactionSucceededToast } from '../../lib/toast';

export const useCreateTransaction = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createTransaction,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      addTransactionSucceededToast();
    },
  });
};
