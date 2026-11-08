import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createRule, updateRule } from '../utils/api';
import type { RuleInput } from '../utils/api';

/** Creates a rule, or updates `id` when given. Applying it to existing transactions changes them,
 * so those are refetched too. */
export const useSaveRule = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, input }: { id?: number; input: RuleInput }) =>
      id === undefined ? createRule(input) : updateRule(id, input),
    onSuccess: (_rule, { input }) => {
      queryClient.invalidateQueries({ queryKey: ['rules'] });
      if (input.apply_to_existing) {
        queryClient.invalidateQueries({ queryKey: ['transactions'] });
        queryClient.invalidateQueries({ queryKey: ['merchants'] });
        queryClient.invalidateQueries({ queryKey: ['tags'] });
      }
    },
  });
};
