import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateCategoryGroup } from '../utils/api';
import type { CategoryGroupPatch } from '../utils/api';

export const useUpdateCategoryGroup = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: CategoryGroupPatch }) =>
      updateCategoryGroup(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['category-groups'] });
    },
  });
};
