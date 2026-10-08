import { useQuery } from '@tanstack/react-query';
import { fetchMerchants } from '../utils/api';
import type { MerchantSortOrder } from '../utils/types';

export const useMerchants = (order?: MerchantSortOrder) =>
  useQuery({
    queryKey: ['merchants', order ?? 'transaction_count'],
    queryFn: () => fetchMerchants(order),
  });
