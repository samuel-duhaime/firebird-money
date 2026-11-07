import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Select } from '../../../components/Select';
import {
  useRemoveMember,
  useUpdateMemberRole,
} from '../hooks/use-household-membership';
import { memberDisplayName } from '../utils/format';
import type { Member } from '../utils/types';
import { MemberPicturePicker } from './MemberPicturePicker';
import '../../transactions/components/AddTransactionModal.css';
import '../../transactions/components/EditTransactionModal.css';
import './MemberEditModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type MemberEditModalProps = {
  member: Member;
  /** The signed-in user is a manager, so they can change roles and remove people. */
  canManage: boolean;
  /** This member is the household's only manager, so they can't become a member. */
  isLastManager: boolean;
  /** This member is the signed-in user, who leaves instead of being removed. */
  isSelf: boolean;
  onClose: () => void;
};

export const MemberEditModal = ({
  member,
  canManage,
  isLastManager,
  isSelf,
  onClose,
}: MemberEditModalProps) => {
  const { t } = useTranslation();
  const updateRole = useUpdateMemberRole();
  const removeMember = useRemoveMember();
  const [confirmingRemove, setConfirmingRemove] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  // The Edit button that opened this: closing removes whatever is focused in here, so focus goes
  // back to it instead of being lost.
  const [returnFocusTarget] = useState<HTMLElement | null>(
    () => document.activeElement as HTMLElement | null,
  );

  const handleClose = () => {
    onClose();
    returnFocusTarget?.focus();
  };

  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      handleClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    ).filter(
      (element) => element.tabIndex !== -1 && !element.matches(':disabled'),
    );
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  const name = memberDisplayName(member);

  const handleRemove = () => {
    removeMember.mutate(member.id, { onSuccess: onClose });
  };

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        ref={dialogRef}
        className="member-edit-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="member-edit-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="member-edit-modal-title">{name}</h2>

          <button
            ref={closeRef}
            type="button"
            className="modal-close"
            onClick={handleClose}
            aria-label={t('transactions.add.close', 'Close')}
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <MemberPicturePicker
            memberId={member.id}
            name={name}
            pictureUrl={member.picture_url}
          />

          {canManage && (
            <div className="member-edit-role">
              <label htmlFor="member-edit-role-select">
                {t('settings.members.role.label')}
              </label>
              <Select
                id="member-edit-role-select"
                value={member.role}
                disabled={isLastManager || updateRole.isPending}
                onChange={(e) =>
                  updateRole.mutate({
                    id: member.id,
                    role: e.target.value as Member['role'],
                  })
                }
              >
                <option value="family_manager">
                  {t('settings.members.manager')}
                </option>
                <option value="family_member">
                  {t('settings.members.member')}
                </option>
              </Select>
              {isLastManager && (
                <span className="member-edit-hint">
                  {t('settings.members.role.lastManager')}
                </span>
              )}
            </div>
          )}
        </div>

        {canManage && !isSelf && (
          <div className="edit-transaction-footer">
            {confirmingRemove ? (
              <div className="delete-confirm">
                <span className="delete-confirm-text">
                  {t('settings.members.remove.confirm', { name })}
                </span>
                <div className="modal-footer-actions">
                  <button
                    type="button"
                    className="modal-button modal-button--cancel"
                    onClick={() => setConfirmingRemove(false)}
                  >
                    {t('transactions.add.cancel')}
                  </button>
                  <button
                    type="button"
                    className="modal-button modal-button--delete-confirm"
                    onClick={handleRemove}
                    disabled={removeMember.isPending}
                  >
                    {t('settings.members.remove.confirmButton')}
                  </button>
                </div>
              </div>
            ) : (
              <button
                type="button"
                className="modal-button modal-button--delete"
                onClick={() => setConfirmingRemove(true)}
              >
                {t('settings.members.remove.button')}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
