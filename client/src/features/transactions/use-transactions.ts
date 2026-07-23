import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../lib/api-client';
import type { Transaction } from './types';

export const useTransactions = (search?: string) =>
  useQuery({
    queryKey: ['transactions', search ?? null],
    queryFn: () =>
      apiFetch<Transaction[]>(
        search ? `/transactions?search=${encodeURIComponent(search)}` : '/transactions',
      ),
  });
