import { useEffect, useId, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
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
  /** For a `<label htmlFor>` to point at the trigger. */
  id?: string;
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

/** Who owns something, shown the same way the picker shows it: the Shared icon and label, or the
 * member's avatar and name. Shows nothing while the members load, and says the owner is
 * unavailable if they fail to (or the owner isn't among them). */
export const OwnerLabel = ({ owner }: { owner: number | null }) => {
  const { t } = useTranslation();
  const { data: members, isPending } = useMembers();

  if (owner === null) {
    return (
      <>
        <SharedIcon />
        <span className="owner-picker-name">
          {t('settings.members.owner.shared')}
        </span>
      </>
    );
  }

  if (isPending) return null;

  const member = members?.find((m) => m.id === owner);
  if (!member) {
    return (
      <span className="owner-picker-name owner-picker-name--placeholder">
        {t('settings.members.owner.unavailable')}
      </span>
    );
  }
  return <MemberLabel member={member} />;
};

/** The keys that move through a closed or open list the way they do in a native `<select>`. */
const NAVIGATION_KEYS = ['ArrowDown', 'ArrowUp', 'Home', 'End'];

/**
 * Picks who owns something: Shared, or one member of the household, each with their avatar.
 * Behaves like a native `<select>` from the keyboard (a select-only combobox): focus stays on the
 * trigger, the arrows, Home and End change the choice while closed or move through the list while
 * open, a letter jumps to the next choice starting with it, Alt+Down, Enter or Space opens, and
 * Enter, Space or Tab picks the highlighted choice, Escape closes without picking.
 */
export const OwnerPicker = ({
  value,
  onChange,
  placeholder,
  id,
  triggerClassName,
  ariaLabel,
}: OwnerPickerProps) => {
  const { t } = useTranslation();
  const { data: members } = useMembers();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const listboxId = useId();
  // The highlighted choice's owner rather than its position, so it stays on the right person if
  // the members load while the list is open.
  const [highlightedOwner, setHighlightedOwner] = useState<number | null>(null);

  const selected = members?.find((member) => member.id === value);
  const sharedLabel = t('settings.members.owner.shared');

  // Every choice in list order, with the text type-to-jump matches against.
  const options: { owner: number | null; label: string; member?: Member }[] = [
    { owner: null, label: sharedLabel },
    ...(members ?? []).map((member) => ({
      owner: member.id,
      label: memberDisplayName(member),
      member,
    })),
  ];
  const selectedIndex = options.findIndex((option) => option.owner === value);
  const highlightedIndex = options.findIndex(
    (option) => option.owner === highlightedOwner,
  );
  const optionId = (index: number) => `${listboxId}-option-${index}`;

  const open = () => {
    setHighlightedOwner(value ?? null);
    setIsOpen(true);
  };

  const highlight = (index: number) => {
    const option = options[index];
    if (option) setHighlightedOwner(option.owner);
  };

  const pick = (index: number) => {
    setIsOpen(false);
    const option = options[index];
    if (option && option.owner !== value) onChange(option.owner);
  };

  /** Where a navigation key or a typed letter moves to from `from` (-1 when nothing is picked). */
  const target = (key: string, from: number) => {
    const last = options.length - 1;
    if (key === 'ArrowDown') return Math.min(from + 1, last);
    if (key === 'ArrowUp') return Math.max(from - 1, 0);
    if (key === 'Home') return 0;
    if (key === 'End') return last;

    // A letter: the next choice starting with it, wrapping around.
    const letter = key.toLocaleLowerCase();
    for (let step = 1; step <= options.length; step++) {
      const index = (from + step + options.length) % options.length;
      if (options[index].label.toLocaleLowerCase().startsWith(letter)) {
        return index;
      }
    }
    return from;
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    const isLetter = event.key.length === 1 && event.key !== ' ';
    const isModified = event.ctrlKey || event.metaKey;

    if (!isOpen) {
      if (event.altKey && event.key === 'ArrowDown') {
        event.preventDefault();
        open();
      } else if (
        (NAVIGATION_KEYS.includes(event.key) || isLetter) &&
        !event.altKey &&
        !isModified
      ) {
        event.preventDefault();
        const index = target(event.key, selectedIndex);
        if (index >= 0) pick(index);
      }
      // Enter and Space click the button, which opens it.
      return;
    }

    if (event.key === 'Escape') {
      // Only the list closes, not a modal the picker sits in.
      event.preventDefault();
      event.stopPropagation();
      setIsOpen(false);
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      pick(highlightedIndex);
    } else if (event.key === 'Tab') {
      // Picks and lets focus move on, like a native select.
      pick(highlightedIndex);
    } else if (event.altKey && event.key === 'ArrowUp') {
      event.preventDefault();
      pick(highlightedIndex);
    } else if (
      NAVIGATION_KEYS.includes(event.key) ||
      (isLetter && !isModified)
    ) {
      event.preventDefault();
      highlight(target(event.key, highlightedIndex));
    }
  };

  // Keeps the highlighted choice in view in a long list.
  useEffect(() => {
    if (!isOpen) return;
    document
      .getElementById(`${listboxId}-option-${highlightedIndex}`)
      ?.scrollIntoView({ block: 'nearest' });
  }, [isOpen, listboxId, highlightedIndex]);

  const isShown = isOpen && position;

  return (
    <>
      <button
        id={id}
        type="button"
        role="combobox"
        className={`owner-picker-trigger ${triggerClassName ?? ''}`}
        ref={triggerRef}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={isShown ? listboxId : undefined}
        aria-activedescendant={
          isShown && highlightedIndex >= 0
            ? optionId(highlightedIndex)
            : undefined
        }
        aria-label={ariaLabel}
        onClick={() => (isOpen ? setIsOpen(false) : open())}
        onKeyDown={handleKeyDown}
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
      {isShown &&
        createPortal(
          <div
            id={listboxId}
            className="anchored-popover owner-picker-popover"
            role="listbox"
            ref={popoverRef}
            // As wide as the field it opens from. Read on each render, and the hook re-renders on
            // resize as it repositions.
            style={{
              top: position.top,
              left: position.left,
              width: triggerRef.current?.offsetWidth,
            }}
          >
            {options.map((option, index) => (
              <div
                key={option.owner ?? 'shared'}
                id={optionId(index)}
                role="option"
                aria-selected={option.owner === value}
                className={
                  index === highlightedIndex
                    ? 'owner-picker-option owner-picker-option--highlighted'
                    : 'owner-picker-option'
                }
                // Keeps focus on the trigger, where the keyboard keeps working.
                onMouseDown={(event) => event.preventDefault()}
                onMouseEnter={() => highlight(index)}
                onClick={() => pick(index)}
              >
                {option.member ? (
                  <MemberLabel member={option.member} />
                ) : (
                  <>
                    <SharedIcon />
                    <span className="owner-picker-name">{sharedLabel}</span>
                  </>
                )}
              </div>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
};
