import { useMutation, useQueryClient } from '@tanstack/react-query';
import { reorderCategories } from '../utils/api';

export const useReorderCategories = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: reorderCategories,
    onSuccess: (categories) => {
      queryClient.setQueryData(['categories'], categories);
    },
  });
};
