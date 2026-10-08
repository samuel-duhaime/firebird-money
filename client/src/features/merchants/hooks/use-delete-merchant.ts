import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteMerchant } from '../utils/api';

export const useDeleteMerchant = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteMerchant,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
    },
  });
};
