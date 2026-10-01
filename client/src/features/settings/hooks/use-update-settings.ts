import { useMutation, useQueryClient } from '@tanstack/react-query';
import { updateSettings } from '../utils/api';

export const useUpdateSettings = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: updateSettings,
    // Writes the server's response straight into the cache instead of invalidating: a column
    // switch reads its checked state from this same query, so waiting on a refetch leaves a window
    // where a second, rapid click on the same switch would still see the pre-toggle value and send
    // it right back (undoing the first click instead of toggling again).
    onSuccess: (settings) => {
      queryClient.setQueryData(['settings'], settings);
    },
  });
};
