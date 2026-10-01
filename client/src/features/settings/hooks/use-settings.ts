import { useQuery } from '@tanstack/react-query';
import { getSettings } from '../utils/api';

export const useSettings = () =>
  useQuery({
    queryKey: ['settings'],
    queryFn: getSettings,
  });
