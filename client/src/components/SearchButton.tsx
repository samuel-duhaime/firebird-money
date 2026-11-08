import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from './TopMenuButton';
import { useAnchoredPopover } from '../lib/use-anchored-popover';
import './Popover.css';
import './SearchButton.css';

type SearchButtonProps = {
  /** The applied search term, `undefined` when there's none. */
  value: string | undefined;
  onChange: (term: string | undefined) => void;
  /** What the term is matched against, shown under the input. */
  help: string;
};

/** A "Search" button that opens a popover to type a term in, applied on Apply or Enter. */
export const SearchButton = ({ value, onChange, help }: SearchButtonProps) => {
  const { t } = useTranslation();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover();
  const [draft, setDraft] = useState(value ?? '');

  useEffect(() => {
    if (isOpen) setDraft(value ?? '');
  }, [isOpen, value]);

  const isDraftEmpty = draft.trim() === '';
  const hasNothingToClear = isDraftEmpty && !value;

  const handleApply = () => {
    if (hasNothingToClear) return;
    onChange(isDraftEmpty ? undefined : draft.trim());
    setIsOpen(false);
  };

  const handleClearDraft = () => {
    if (hasNothingToClear) return;
    setDraft('');
    onChange(undefined);
    setIsOpen(false);
  };

  const handleCancel = () => {
    setDraft(value ?? '');
    setIsOpen(false);
  };

  const handleInputKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') handleApply();
  };

  return (
    <div className="search-button-trigger" ref={triggerRef}>
      <TopMenuButton
        icon={faMagnifyingGlass}
        label={value ? `"${value}"` : t('search.trigger')}
        onClick={() => setIsOpen((open) => !open)}
      />
      {value && <span className="search-button-badge" />}
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover search-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <h4 className="search-popover-title">{t('search.title')}</h4>
            <input
              type="text"
              className="search-popover-input"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={handleInputKeyDown}
              placeholder={t('search.placeholder')}
              aria-label={t('search.title')}
              autoFocus
            />
            <p className="search-popover-help">{help}</p>
            <div className="search-popover-actions">
              <button
                type="button"
                className="search-popover-button"
                disabled={hasNothingToClear}
                onClick={handleClearDraft}
              >
                {t('search.clear')}
              </button>
              <button
                type="button"
                className="search-popover-button"
                onClick={handleCancel}
              >
                {t('search.cancel')}
              </button>
              <button
                type="button"
                className="search-popover-button search-popover-button--primary"
                disabled={hasNothingToClear}
                onClick={handleApply}
              >
                {t('search.apply')}
              </button>
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
};
