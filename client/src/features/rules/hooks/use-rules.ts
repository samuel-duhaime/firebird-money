import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../../lib/api-client';
import type { Rule } from '../utils/types';

export const useRules = () =>
  useQuery({
    queryKey: ['rules'],
    queryFn: () => apiFetch<Rule[]>('/rules'),
  });
