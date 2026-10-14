import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createInstitution } from '../utils/api';

export const useCreateInstitution = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createInstitution,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['institutions'] });
    },
  });
};
