import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateInstitution } from '../utils/api';
import type { InstitutionPatch } from '../utils/api';

export const useUpdateInstitution = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, patch }: { id: number; patch: InstitutionPatch }) =>
      updateInstitution(id, patch),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['institutions'] });
      // Each account carries its institution's joined name.
      queryClient.invalidateQueries({ queryKey: ['accounts'] });
    },
  });
};
