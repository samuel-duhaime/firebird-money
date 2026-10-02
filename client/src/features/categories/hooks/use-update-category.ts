import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateCategory } from '../utils/api';
import type { CategoryPatch } from '../utils/api';

export const useUpdateCategory = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: CategoryPatch }) =>
      updateCategory(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['categories'] });
      // A transaction's category name/type are joined fresh on every fetch, but a cached
      // transactions list can still go on showing a category's old name for up to the query's
      // staleTime after it's edited here — see the same note on `useUpdateTag`.
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
