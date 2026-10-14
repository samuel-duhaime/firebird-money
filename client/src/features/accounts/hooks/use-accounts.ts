import { useQuery } from '@tanstack/react-query';
import { fetchAccounts } from '../utils/api';

export const useAccounts = () =>
  useQuery({
    queryKey: ['accounts'],
    queryFn: fetchAccounts,
  });
