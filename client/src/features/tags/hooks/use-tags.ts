import { useQuery } from '@tanstack/react-query';
import { apiFetch } from '../../../lib/api-client';
import type { Tag } from '../utils/types';

export const useTags = () =>
  useQuery({
    queryKey: ['tags'],
    queryFn: () => apiFetch<Tag[]>('/tags'),
  });
