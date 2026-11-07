import { useQuery } from '@tanstack/react-query';
import { fetchAccounts } from '../../accounts/utils/api';

/**
 * How many transactions the household has across all its accounts, refetched when the caller
 * mounts so a cached count can't hide a transaction added since. `null` while that refetch runs
 * or if it fails: the count is then unknown, and the download stays required.
 */
export const useHouseholdTransactionCount = () => {
  const {
    data: accounts,
    isFetching,
    isError,
  } = useQuery({
    queryKey: ['accounts'],
    queryFn: fetchAccounts,
    refetchOnMount: 'always',
  });

  if (isFetching || isError || !accounts) return null;
  return accounts.reduce((sum, account) => sum + account.transaction_count, 0);
};
