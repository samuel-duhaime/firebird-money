import { useMutation, useQueryClient } from '@tanstack/react-query';
import { deleteInstitution } from '../utils/api';

export const useDeleteInstitution = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: deleteInstitution,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['institutions'] });
    },
  });
};
