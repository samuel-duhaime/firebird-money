import { useQuery } from '@tanstack/react-query';
import { fetchInstitutions } from '../utils/api';

export const useInstitutions = () =>
  useQuery({
    queryKey: ['institutions'],
    queryFn: fetchInstitutions,
  });
