import { useMutation, useQueryClient } from '@tanstack/react-query';
import { reorderRules } from '../utils/api';

export const useReorderRules = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: reorderRules,
    onSuccess: (rules) => {
      queryClient.setQueryData(['rules'], rules);
    },
  });
};
