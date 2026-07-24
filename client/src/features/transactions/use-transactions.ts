import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../lib/api-client';
import type { SortOrder, Transaction } from './types';

export const useTransactions = (search?: string, order?: SortOrder) =>
  useQuery({
    queryKey: ['transactions', search ?? null, order ?? null],
    queryFn: () => {
      const params = new URLSearchParams();
      if (search) params.set('search', search);
      if (order) params.set('order', order);
      const query = params.toString();
      return apiFetch<Transaction[]>(`/transactions${query ? `?${query}` : ''}`);
    },
  });
