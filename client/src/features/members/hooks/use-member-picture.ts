import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ApiError } from '../../../lib/api-client';
import {
  deleteMemberPictureFailedToast,
  memberPictureUnreadableToast,
  uploadMemberPictureFailedToast,
} from '../../../lib/toast';
import { currentUserQueryKey } from '../../auth/hooks/use-current-user';
import { deleteMemberPicture, uploadMemberPicture } from '../utils/api';
import { UnreadablePictureError, resizePicture } from '../utils/picture';

/** Shared by both mutations, so a screen can tell when either is running (`useIsMutating`). */
export const memberPictureMutationKey = ['member-picture'] as const;

/** A picture shows in the members list and, for the signed-in user, everywhere their session does. */
const useRefreshPictures = () => {
  const queryClient = useQueryClient();

  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ['members'] }),
      queryClient.invalidateQueries({ queryKey: currentUserQueryKey }),
    ]);
};

export const useUploadMemberPicture = () => {
  const refreshPictures = useRefreshPictures();

  return useMutation({
    mutationKey: memberPictureMutationKey,
    mutationFn: async ({ id, file }: { id: number; file: File }) =>
      uploadMemberPicture(id, await resizePicture(file)),
    onSuccess: refreshPictures,
    onError: (error: Error) => {
      if (
        error instanceof UnreadablePictureError ||
        (error instanceof ApiError && error.status === 400)
      ) {
        memberPictureUnreadableToast();
        return;
      }
      uploadMemberPictureFailedToast();
    },
  });
};

export const useDeleteMemberPicture = () => {
  const refreshPictures = useRefreshPictures();

  return useMutation({
    mutationKey: memberPictureMutationKey,
    mutationFn: deleteMemberPicture,
    onSuccess: refreshPictures,
    onError: deleteMemberPictureFailedToast,
  });
};
