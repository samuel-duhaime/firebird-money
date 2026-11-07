import { useAccounts } from '../../accounts/hooks/use-accounts';

/** How many transactions the household has across all its accounts, or `null` while loading. */
export const useHouseholdTransactionCount = () => {
  const { data: accounts } = useAccounts();

  return (
    accounts?.reduce((sum, account) => sum + account.transaction_count, 0) ??
    null
  );
};
