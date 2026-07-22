import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../lib/api-client';
import type { Transaction } from './types';

export const useTransactions = () =>
  useQuery({
    queryKey: ['transactions'],
    queryFn: () => apiFetch<Transaction[]>('/transactions'),
  });
