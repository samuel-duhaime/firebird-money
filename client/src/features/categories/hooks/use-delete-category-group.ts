import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteCategoryGroup } from '../utils/api';

export const useDeleteCategoryGroup = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteCategoryGroup,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['category-groups'] });
    },
  });
};
