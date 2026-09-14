import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../../lib/api-client';
import type { CategoryGroup } from '../utils/types';

export const useCategoryGroups = () =>
  useQuery({
    queryKey: ['category-groups'],
    queryFn: () => apiFetch<CategoryGroup[]>('/category-groups'),
  });
