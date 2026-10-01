import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createTag } from '../utils/api';

export const useCreateTag = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createTag,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tags'] });
    },
  });
};
