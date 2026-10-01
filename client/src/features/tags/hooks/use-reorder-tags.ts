import { useMutation, useQueryClient } from '@tanstack/react-query';
import { reorderTags } from '../utils/api';

export const useReorderTags = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: reorderTags,
    onSuccess: (tags) => {
      queryClient.setQueryData(['tags'], tags);
    },
  });
};
