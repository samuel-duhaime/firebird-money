import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteTag } from '../utils/api';

export const useDeleteTag = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteTag,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tags'] });
    },
  });
};
