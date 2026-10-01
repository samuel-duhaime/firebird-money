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
      // A transaction's `tags` are embedded at fetch time (name/color snapshot, not a live
      // reference) — without this, a cached transactions list can go on showing a tag's old name
      // or color for up to the query's staleTime after it's edited here.
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
    },
  });
};
