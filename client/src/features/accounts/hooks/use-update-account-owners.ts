import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateAccountOwners } from '../utils/api';
import type { AccountOwner } from '../utils/api';

export const useUpdateAccountOwners = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (owners: AccountOwner[]) => updateAccountOwners(owners),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
      // The accounts' transactions follow their new owner.
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
