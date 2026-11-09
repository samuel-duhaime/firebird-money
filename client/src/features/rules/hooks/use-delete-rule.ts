import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteRule } from '../utils/api';

export const useDeleteRule = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteRule,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rules'] });
    },
  });
};
