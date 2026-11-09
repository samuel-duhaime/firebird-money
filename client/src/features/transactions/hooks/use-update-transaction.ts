import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { QueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { updateTransaction } from '../utils/api';
import type { TransactionPatch } from '../utils/api';
import type { Transaction } from '../utils/types';
import { ruleSuggestionToast } from '../../rules/utils/rule-suggestion';

/** The transaction `id` as it's cached right now (its own query, or any list holding it). */
const cachedTransaction = (
  queryClient: QueryClient,
  id: number,
): Transaction | undefined => {
  const own = queryClient.getQueryData<Transaction>(['transactions', id]);
  if (own) return own;
  for (const [, data] of queryClient.getQueriesData<unknown>({
    queryKey: ['transactions'],
  })) {
    if (Array.isArray(data)) {
      const found = (data as Transaction[]).find((t) => t.id === id);
      if (found) return found;
    }
  }
  return undefined;
};

export const useUpdateTransaction = () => {
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: TransactionPatch }) =>
      updateTransaction(id, patch),
    // What it was before, for the "Create rule" suggestion: a rule matches the old merchant.
    onMutate: ({ id }) => ({ before: cachedTransaction(queryClient, id) }),
    onSuccess: (updated, { patch }, context) => {
      queryClient.invalidateQueries({ queryKey: ['transactions'] });
      // A merchant's transaction_count/recommended_category_id are computed from the household's
      // transactions, so changing one (especially its merchant_id or category_id) can change
      // either for the old and/or new merchant.
      queryClient.invalidateQueries({ queryKey: ['merchants'] });
      if (context?.before) {
        ruleSuggestionToast(context.before, updated, patch, () =>
          navigate({ to: '/settings/rules' }),
        );
      }
    },
  });
};
