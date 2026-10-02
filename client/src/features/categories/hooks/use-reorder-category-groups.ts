import { useMutation, useQueryClient } from '@tanstack/react-query';
import { reorderCategoryGroups } from '../utils/api';

export const useReorderCategoryGroups = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: reorderCategoryGroups,
    onSuccess: (groups) => {
      queryClient.setQueryData(['category-groups'], groups);
    },
  });
};
