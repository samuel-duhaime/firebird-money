import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateMerchant } from '../utils/api';
import type { MerchantPatch } from '../utils/api';

export const useUpdateMerchant = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: MerchantPatch }) =>
      updateMerchant(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
      // A transaction's joined `merchant_name` can go stale in the cache for up to the query's
      // staleTime after a rename here — same reasoning as `useUpdateCategory`/`useUpdateTag`.
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
