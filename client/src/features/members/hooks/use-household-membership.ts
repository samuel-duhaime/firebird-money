import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  deleteHouseholdFailedToast,
  leaveHouseholdFailedToast,
  regenerateJoinCodeFailedToast,
  removeMemberFailedToast,
  updateMemberRoleFailedToast,
} from '../../../lib/toast';
import { currentUserQueryKey } from '../../auth/hooks/use-current-user';
import {
  deleteHousehold,
  leaveHousehold,
  regenerateJoinCode,
  removeMember,
  updateMemberRole,
} from '../utils/api';
import type { Member } from '../utils/types';

/** Roles and the invite code show in the members list and in the signed-in user's session. */
const useRefreshMembers = () => {
  const queryClient = useQueryClient();

  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ['members'] }),
      queryClient.invalidateQueries({ queryKey: currentUserQueryKey }),
    ]);
};

export const useUpdateMemberRole = () => {
  const refreshMembers = useRefreshMembers();

  return useMutation({
    mutationFn: ({ id, role }: { id: number; role: Member['role'] }) =>
      updateMemberRole(id, role),
    onSuccess: refreshMembers,
    onError: updateMemberRoleFailedToast,
  });
};

/** Removing someone also changes the invite code, and makes what they owned Shared. */
export const useRemoveMember = () => {
  const queryClient = useQueryClient();
  const refreshMembers = useRefreshMembers();

  return useMutation({
    mutationFn: removeMember,
    onSuccess: () =>
      Promise.all([
        refreshMembers(),
        queryClient.invalidateQueries({ queryKey: ['accounts'] }),
        queryClient.invalidateQueries({ queryKey: ['transactions'] }),
      ]),
    onError: removeMemberFailedToast,
  });
};

export const useRegenerateJoinCode = () => {
  const refreshMembers = useRefreshMembers();

  return useMutation({
    mutationFn: regenerateJoinCode,
    onSuccess: refreshMembers,
    onError: regenerateJoinCodeFailedToast,
  });
};

/** After leaving or deleting, nothing cached belongs to the user anymore: start over from the
 * session, which sends them back to onboarding's household step. */
const useStartOver = () => {
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  return async () => {
    queryClient.removeQueries({
      predicate: (query) => query.queryKey[0] !== currentUserQueryKey[0],
    });
    await queryClient.refetchQueries({ queryKey: currentUserQueryKey });
    await navigate({ to: '/onboarding' });
  };
};

export const useLeaveHousehold = () => {
  const startOver = useStartOver();

  return useMutation({
    mutationFn: leaveHousehold,
    onSuccess: startOver,
    onError: leaveHouseholdFailedToast,
  });
};

export const useDeleteHousehold = () => {
  const startOver = useStartOver();

  return useMutation({
    mutationFn: deleteHousehold,
    onSuccess: startOver,
    onError: deleteHouseholdFailedToast,
  });
};
