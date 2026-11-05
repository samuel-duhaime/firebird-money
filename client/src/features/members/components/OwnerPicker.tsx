import { useEffect, useId, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faLink } from '@fortawesome/free-solid-svg-icons';
import type { IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { Avatar } from '../../../components/Avatar';
import { Tooltip } from '../../../components/Tooltip';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import { useMembers } from '../hooks/use-members';
import { memberDisplayName, pictureSrc } from '../utils/format';
import type { Member } from '../utils/types';
import '../../../components/Popover.css';
import './OwnerPicker.css';

/** A choice that isn't a member, e.g. Shared, listed before the members with its own icon. */
export type PickerChoice<V> = { value: V; label: string; icon: ReactNode };

type OwnerPickerProps<V extends string | null> = {
  /** A member's id, one of `choices`' values, or `undefined` while nothing is picked yet. */
  value: number | V | undefined;
  onChange: (value: number | V) => void;
  /** The choices besides the members. Defaults to Shared (`null`). */
  choices?: PickerChoice<V>[];
  /** Shown while `value` is `undefined`. */
  placeholder?: string;
  /** The owner it gets when nobody picks one, e.g. a transaction's account's: marked "(default)"
   * wherever it shows. */
  defaultOwner?: number | V;
  disabled?: boolean;
  /** `field` (default) is a form field; `cell` is only the avatar, for a list row, with the name
   * and email on hover. */
  variant?: 'field' | 'cell';
  /** For a `<label htmlFor>` to point at the trigger. */
  id?: string;
  triggerClassName?: string;
  ariaLabel?: string;
};

/** Stands in for an avatar wherever a choice isn't a member, e.g. Shared. */
export const ChoiceIcon = ({ icon }: { icon: IconDefinition }) => (
  <span className="owner-picker-shared-icon" aria-hidden="true">
    <FontAwesomeIcon icon={icon} />
  </span>
);

/** Stands in for an avatar wherever Shared is a choice. */
export const SharedIcon = () => <ChoiceIcon icon={faLink} />;

/** The name (with any `suffix`), and the email when the name isn't already it — what a lone
 * avatar shows on hover. */
const memberTooltip = (member: Member, suffix = '') => {
  const name = memberDisplayName(member);
  return name === member.email
    ? name + suffix
    : `${name}${suffix}\n${member.email}`;
};

const MemberLabel = ({
  member,
  suffix,
}: {
  member: Member;
  suffix?: string;
}) => {
  const name = memberDisplayName(member);

  return (
    <>
      <Avatar name={name} src={pictureSrc(member.picture_url)} />
      <span className="owner-picker-name">
        {name}
        {suffix}
      </span>
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

/** Who owns something as just its avatar (or the Shared icon), with the name and email on hover —
 * for a narrow list column. */
export const OwnerAvatar = ({ owner }: { owner: number | null }) => {
  const { t } = useTranslation();
  const { data: members } = useMembers();

  if (owner === null) {
    return (
      <Tooltip text={t('settings.members.owner.shared')}>
        <span className="owner-avatar">
          <SharedIcon />
        </span>
      </Tooltip>
    );
  }
  const member = members?.find((m) => m.id === owner);
  if (!member) return null;
  return (
    <Tooltip text={memberTooltip(member)}>
      <span className="owner-avatar">
        <Avatar
          name={memberDisplayName(member)}
          src={pictureSrc(member.picture_url)}
        />
      </span>
    </Tooltip>
  );
};

/** The keys that move through a closed or open list the way they do in a native `<select>`. */
const NAVIGATION_KEYS = ['ArrowDown', 'ArrowUp', 'Home', 'End'];

/**
 * Picks who owns something: Shared, or one member of the household, each with their avatar. Other
 * `choices` can stand in for Shared, e.g. who a review waits on.
 * Behaves like a native `<select>` from the keyboard (a select-only combobox): focus stays on the
 * trigger, the arrows, Home and End change the choice while closed or move through the list while
 * open, a letter jumps to the next choice starting with it, Alt+Down, Enter or Space opens, and
 * Enter, Space or Tab picks the highlighted choice, Escape closes without picking.
 */
export const OwnerPicker = <V extends string | null = null>({
  value,
  onChange,
  choices,
  placeholder,
  defaultOwner,
  disabled = false,
  variant = 'field',
  id,
  triggerClassName,
  ariaLabel,
}: OwnerPickerProps<V>) => {
  const { t } = useTranslation();
  const { data: members } = useMembers();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const listboxId = useId();
  // The highlighted choice's value rather than its position, so it stays on the right person if
  // the members load while the list is open.
  const [highlightedOwner, setHighlightedOwner] = useState<
    number | V | undefined
  >(undefined);

  const extraChoices: PickerChoice<V>[] = choices ?? [
    {
      value: null as V,
      label: t('settings.members.owner.shared'),
      icon: <SharedIcon />,
    },
  ];
  const selected = members?.find((member) => member.id === value);
  const selectedChoice = extraChoices.find((choice) => choice.value === value);
  /** " (default)" after the owner that's the default, nothing after the others. */
  const suffix = (owner: number | V | undefined) =>
    defaultOwner !== undefined && owner === defaultOwner
      ? ` ${t('settings.members.owner.default')}`
      : '';
  const isCell = variant === 'cell';
  const tooltip = !isCell
    ? undefined
    : selectedChoice
      ? selectedChoice.label + suffix(selectedChoice.value)
      : selected && memberTooltip(selected, suffix(selected.id));

  // Every choice in list order, with the text type-to-jump matches against.
  const options: {
    owner: number | V;
    label: string;
    icon?: ReactNode;
    member?: Member;
  }[] = [
    ...extraChoices.map((choice) => ({
      owner: choice.value,
      label: choice.label,
      icon: choice.icon,
    })),
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
    setHighlightedOwner(value === undefined ? options[0]?.owner : value);
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

  const trigger = (
    <button
      id={id}
      type="button"
      role="combobox"
      className={`owner-picker-trigger owner-picker-trigger--${variant} ${triggerClassName ?? ''}`}
      ref={triggerRef}
      disabled={disabled}
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-controls={isShown ? listboxId : undefined}
      aria-activedescendant={
        isShown && highlightedIndex >= 0
          ? optionId(highlightedIndex)
          : undefined
      }
      aria-label={isCell && tooltip ? `${ariaLabel}: ${tooltip}` : ariaLabel}
      onClick={() => (isOpen ? setIsOpen(false) : open())}
      onKeyDown={handleKeyDown}
    >
      {selectedChoice && (
        <>
          {selectedChoice.icon}
          {!isCell && (
            <span className="owner-picker-name">
              {selectedChoice.label}
              {suffix(selectedChoice.value)}
            </span>
          )}
        </>
      )}
      {selected &&
        (isCell ? (
          <Avatar
            name={memberDisplayName(selected)}
            src={pictureSrc(selected.picture_url)}
          />
        ) : (
          <MemberLabel member={selected} suffix={suffix(selected.id)} />
        ))}
      {value === undefined && (
        <span className="owner-picker-name owner-picker-name--placeholder">
          {placeholder}
        </span>
      )}
      {!isCell && (
        <FontAwesomeIcon
          icon={faChevronDown}
          className="owner-picker-chevron"
        />
      )}
    </button>
  );

  return (
    <>
      {isCell ? <Tooltip text={tooltip}>{trigger}</Tooltip> : trigger}
      {isShown &&
        createPortal(
          <div
            id={listboxId}
            className="anchored-popover owner-picker-popover"
            role="listbox"
            ref={popoverRef}
            // As wide as the field it opens from (a cell's is narrower than its names, so the
            // popover's min-width takes over). Read on each render, and the hook re-renders on
            // resize as it repositions.
            style={{
              top: position.top,
              left: position.left,
              width: isCell ? undefined : triggerRef.current?.offsetWidth,
            }}
          >
            {options.map((option, index) => (
              <div
                key={option.owner ?? 'null'}
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
                  <MemberLabel
                    member={option.member}
                    suffix={suffix(option.owner)}
                  />
                ) : (
                  <>
                    {option.icon}
                    <span className="owner-picker-name">
                      {option.label}
                      {suffix(option.owner)}
                    </span>
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
