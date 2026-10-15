import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../../lib/api-client';
import type { Member } from '../utils/types';

export const useMembers = () =>
  useQuery({
    queryKey: ['members'],
    queryFn: () => apiFetch<Member[]>('/members'),
  });
