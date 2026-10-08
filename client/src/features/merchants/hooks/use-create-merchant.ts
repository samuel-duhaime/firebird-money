import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createMerchant } from '../utils/api';

export const useCreateMerchant = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createMerchant,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
    },
  });
};
