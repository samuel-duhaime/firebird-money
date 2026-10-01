import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateTag } from '../utils/api';
import type { TagPatch } from '../utils/api';

export const useUpdateTag = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: TagPatch }) =>
      updateTag(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tags'] });
    },
  });
};
