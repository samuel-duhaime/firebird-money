import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../lib/api-client';
import type { Category } from './types';

export const useCategories = () =>
  useQuery({
    queryKey: ['categories'],
    queryFn: () => apiFetch<Category[]>('/categories'),
  });
