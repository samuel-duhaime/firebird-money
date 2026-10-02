import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createCategoryGroup } from '../utils/api';

export const useCreateCategoryGroup = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createCategoryGroup,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['category-groups'] });
    },
  });
};
