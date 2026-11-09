import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faXmark } from '@fortawesome/free-solid-svg-icons';
import { useAnchoredPopover } from '../lib/use-anchored-popover';
import './Popover.css';
import './MultiSelect.css';

export type MultiSelectOption<V extends string | number> = {
  value: V;
  label: string;
  /** Shown before the label, in the list and on the chip — e.g. a tag's color dot or an avatar. */
  icon?: ReactNode;
  /** A heading the option is listed under, e.g. a category's group. Options sharing one should be
   * next to each other. */
  group?: string;
};

type MultiSelectProps<V extends string | number> = {
  options: MultiSelectOption<V>[];
  selected: V[];
  onChange: (selected: V[]) => void;
  /** Shown while nothing is picked. */
  placeholder: string;
  ariaLabel: string;
  /** The trigger's look, e.g. `modal-category-trigger` in a form. */
  triggerClassName?: string;
};

/**
 * Picks any number of `options`: the picked ones show as removable chips on the trigger, and the
 * popover lists every option (searchable) with a checkbox. Use this for any "one or more of" field
 * instead of styling a list of checkboxes per screen.
 */
export const MultiSelect = <V extends string | number>({
  options,
  selected,
  onChange,
  placeholder,
  ariaLabel,
  triggerClassName,
}: MultiSelectProps<V>) => {
  const { t } = useTranslation();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLDivElement>();
  const [search, setSearch] = useState('');
  const searchRef = useRef<HTMLInputElement>(null);

  const isShown = isOpen && position !== null;
  useEffect(() => {
    if (!isShown) return;
    setSearch('');
    // The popover (and its search) only mounts once it has a position.
    searchRef.current?.focus();
  }, [isShown]);

  const filtered = useMemo(() => {
    const term = search.trim().toLowerCase();
    if (term === '') return options;
    return options.filter((option) =>
      option.label.toLowerCase().includes(term),
    );
  }, [options, search]);

  const picked = options.filter((option) => selected.includes(option.value));

  const toggle = (value: V) =>
    onChange(
      selected.includes(value)
        ? selected.filter((v) => v !== value)
        : [...selected, value],
    );

  return (
    <>
      <div
        ref={triggerRef}
        role="button"
        tabIndex={0}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        className={`multi-select-trigger ${triggerClassName ?? ''}`}
        onClick={() => setIsOpen((open) => !open)}
        onKeyDown={(event) => {
          // A chip's remove button handles its own Enter and Space.
          if (event.target !== event.currentTarget) return;
          if (
            event.key === 'Enter' ||
            event.key === ' ' ||
            event.key === 'ArrowDown'
          ) {
            event.preventDefault();
            setIsOpen(true);
          }
        }}
      >
        {picked.length > 0 ? (
          <span className="multi-select-chips">
            {picked.map((option) => (
              <span key={option.value} className="multi-select-chip">
                {option.icon}
                {option.label}
                <button
                  type="button"
                  className="multi-select-chip-remove"
                  aria-label={t('multiSelect.remove', { name: option.label })}
                  onClick={(event) => {
                    event.stopPropagation();
                    toggle(option.value);
                  }}
                >
                  <FontAwesomeIcon icon={faXmark} />
                </button>
              </span>
            ))}
          </span>
        ) : (
          <span className="multi-select-placeholder">{placeholder}</span>
        )}
        <FontAwesomeIcon
          icon={faChevronDown}
          className="multi-select-chevron"
          aria-hidden="true"
        />
      </div>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover multi-select-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <input
              ref={searchRef}
              type="text"
              className="multi-select-search"
              placeholder={t('multiSelect.search')}
              aria-label={t('multiSelect.search')}
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
            <div
              className="multi-select-options"
              role="listbox"
              aria-multiselectable="true"
            >
              {filtered.length === 0 && (
                <p className="multi-select-empty">
                  {t('multiSelect.noResults')}
                </p>
              )}
              {filtered.map((option, index) => (
                <div key={option.value} role="presentation">
                  {option.group !== undefined &&
                    option.group !== filtered[index - 1]?.group && (
                      <p className="multi-select-group">{option.group}</p>
                    )}
                  <label
                    className="multi-select-option"
                    role="option"
                    aria-selected={selected.includes(option.value)}
                  >
                    <input
                      type="checkbox"
                      checked={selected.includes(option.value)}
                      onChange={() => toggle(option.value)}
                    />
                    {option.icon}
                    <span className="multi-select-option-label">
                      {option.label}
                    </span>
                  </label>
                </div>
              ))}
            </div>
          </div>,
          document.body,
        )}
    </>
  );
};
