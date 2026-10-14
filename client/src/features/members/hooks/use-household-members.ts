import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../../lib/api-client';
import type { HouseholdMember } from '../utils/types';

export const useHouseholdMembers = () =>
  useQuery({
    queryKey: ['household-members'],
    queryFn: () => apiFetch<HouseholdMember[]>('/household-members'),
  });
