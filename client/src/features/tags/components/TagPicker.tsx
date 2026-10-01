import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown } from '@fortawesome/free-solid-svg-icons';
import { useTags } from '../hooks/use-tags';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import '../../../components/Popover.css';
import './TagPicker.css';

type TagPickerProps = {
  selectedTagIds: number[];
  /** Toggles a single tag in/out of `selectedTagIds` — the caller decides what that means (set
   * immediately, or buffer until a later Save), so this never closes the popover itself. */
  onToggle: (tagId: number) => void;
  triggerClassName?: string;
  ariaLabel?: string;
};

export const TagPicker = ({
  selectedTagIds,
  onToggle,
  triggerClassName,
  ariaLabel,
}: TagPickerProps) => {
  const { t } = useTranslation();
  const { data: tags } = useTags();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();

  const selected = (tags ?? []).filter((tag) =>
    selectedTagIds.includes(tag.id),
  );

  return (
    <>
      <button
        type="button"
        className={`tag-picker-trigger ${triggerClassName ?? ''}`}
        ref={triggerRef}
        onClick={() => setIsOpen((open) => !open)}
        aria-label={ariaLabel}
      >
        {selected.length > 0 ? (
          <span className="tag-picker-trigger-chips">
            {selected.map((tag) => (
              <span key={tag.id} className="tag-picker-chip">
                <span
                  className="tag-picker-chip-color"
                  style={{ backgroundColor: tag.color }}
                  aria-hidden="true"
                />
                {tag.name}
              </span>
            ))}
          </span>
        ) : (
          <span className="tag-picker-trigger-label tag-picker-trigger-label--placeholder">
            {t('transactions.add.selectTags')}
          </span>
        )}
        <FontAwesomeIcon
          icon={faChevronDown}
          className="tag-picker-trigger-chevron"
        />
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover tag-picker-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            {(!tags || tags.length === 0) && (
              <p className="tag-picker-empty">
                {t('transactions.add.noTagsFound')}
              </p>
            )}
            {tags?.map((tag) => {
              const isSelected = selectedTagIds.includes(tag.id);
              return (
                <button
                  key={tag.id}
                  type="button"
                  className={
                    isSelected
                      ? 'tag-picker-option tag-picker-option--selected'
                      : 'tag-picker-option'
                  }
                  onClick={() => onToggle(tag.id)}
                >
                  <span
                    className="tag-picker-option-color"
                    style={{ backgroundColor: tag.color }}
                    aria-hidden="true"
                  />
                  {tag.name}
                </button>
              );
            })}
          </div>,
          document.body,
        )}
    </>
  );
};
