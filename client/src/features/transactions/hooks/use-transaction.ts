import { useQuery } from '@tanstack/react-query';
import { getTransaction } from '../utils/api';

export const useTransaction = (id: number) =>
  useQuery({
    queryKey: ['transactions', id],
    queryFn: () => getTransaction(id),
    // A 404 (deleted or bad id) will never succeed on retry — fail fast instead of showing
    // "Loading…" through three retries' worth of backoff.
    retry: false,
  });
