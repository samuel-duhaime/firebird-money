import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faLink } from '@fortawesome/free-solid-svg-icons';
import { Avatar } from '../../../components/Avatar';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import { useMembers } from '../hooks/use-members';
import { memberDisplayName, pictureSrc } from '../utils/format';
import type { Member } from '../utils/types';
import '../../../components/Popover.css';
import './OwnerPicker.css';

type OwnerPickerProps = {
  /** The owning member's id, `null` for Shared, or `undefined` while nothing is picked yet. */
  value: number | null | undefined;
  onChange: (owner: number | null) => void;
  /** Shown while `value` is `undefined`. */
  placeholder?: string;
  triggerClassName?: string;
  ariaLabel?: string;
};

const SharedIcon = () => (
  <span className="owner-picker-shared-icon" aria-hidden="true">
    <FontAwesomeIcon icon={faLink} />
  </span>
);

const MemberLabel = ({ member }: { member: Member }) => {
  const name = memberDisplayName(member);

  return (
    <>
      <Avatar name={name} src={pictureSrc(member.picture_url)} />
      <span className="owner-picker-name">{name}</span>
    </>
  );
};

/** Picks who owns something: Shared, or one member of the household, each with their avatar. */
export const OwnerPicker = ({
  value,
  onChange,
  placeholder,
  triggerClassName,
  ariaLabel,
}: OwnerPickerProps) => {
  const { t } = useTranslation();
  const { data: members } = useMembers();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();

  const selected = members?.find((member) => member.id === value);
  const sharedLabel = t('settings.members.owner.shared');

  const pick = (owner: number | null) => {
    setIsOpen(false);
    onChange(owner);
  };

  return (
    <>
      <button
        type="button"
        className={`owner-picker-trigger ${triggerClassName ?? ''}`}
        ref={triggerRef}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-label={ariaLabel}
        onClick={() => setIsOpen((open) => !open)}
      >
        {value === null && (
          <>
            <SharedIcon />
            <span className="owner-picker-name">{sharedLabel}</span>
          </>
        )}
        {selected && <MemberLabel member={selected} />}
        {value === undefined && (
          <span className="owner-picker-name owner-picker-name--placeholder">
            {placeholder}
          </span>
        )}
        <FontAwesomeIcon
          icon={faChevronDown}
          className="owner-picker-chevron"
        />
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover owner-picker-popover"
            role="listbox"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <button
              type="button"
              role="option"
              aria-selected={value === null}
              className="owner-picker-option"
              onClick={() => pick(null)}
            >
              <SharedIcon />
              <span className="owner-picker-name">{sharedLabel}</span>
            </button>
            {members?.map((member) => (
              <button
                key={member.id}
                type="button"
                role="option"
                aria-selected={value === member.id}
                className="owner-picker-option"
                onClick={() => pick(member.id)}
              >
                <MemberLabel member={member} />
              </button>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
};
