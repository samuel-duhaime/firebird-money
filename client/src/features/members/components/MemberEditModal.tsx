import { useEffect, useRef } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { memberDisplayName } from '../utils/format';
import type { Member } from '../utils/types';
import { MemberPicturePicker } from './MemberPicturePicker';
import '../../transactions/components/AddTransactionModal.css';
import './MemberEditModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type MemberEditModalProps = {
  member: Member;
  onClose: () => void;
};

export const MemberEditModal = ({ member, onClose }: MemberEditModalProps) => {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    ).filter((element) => element.tabIndex !== -1);
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

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
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
            onClick={onClose}
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
        </div>
      </div>
    </div>
  );
};
