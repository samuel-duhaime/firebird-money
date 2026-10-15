import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../../lib/api-client';
import { transactionsQueryParams } from '../utils/filters';
import type { TransactionsQuery } from '../utils/filters';
import type { Transaction } from '../utils/types';

export const useTransactions = (query: TransactionsQuery) => {
  const params = transactionsQueryParams(query).toString();
  return useQuery({
    queryKey: ['transactions', params],
    queryFn: () =>
      apiFetch<Transaction[]>(`/transactions${params ? `?${params}` : ''}`),
  });
};
