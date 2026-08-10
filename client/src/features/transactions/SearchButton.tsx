import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import '../../components/Popover.css';
import './SearchButton.css';

const routeApi = getRouteApi('/_app/transactions');

export const SearchButton = () => {
  const { t } = useTranslation();
  const { search } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } = useAnchoredPopover();
  const [draft, setDraft] = useState(search ?? '');

  useEffect(() => {
    if (isOpen) setDraft(search ?? '');
  }, [isOpen, search]);

  const applySearch = (term: string | undefined) => {
    navigate({ search: (prev) => ({ ...prev, search: term }), replace: true });
  };

  const isDraftEmpty = draft.trim() === '';
  const hasNothingToClear = isDraftEmpty && !search;

  const handleApply = () => {
    if (hasNothingToClear) return;
    applySearch(isDraftEmpty ? undefined : draft.trim());
    setIsOpen(false);
  };

  const handleClearDraft = () => {
    if (hasNothingToClear) return;
    setDraft('');
    applySearch(undefined);
    setIsOpen(false);
  };

  const handleCancel = () => {
    setDraft(search ?? '');
    setIsOpen(false);
  };

  const handleInputKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') handleApply();
  };

  return (
    <div className="search-button-trigger" ref={triggerRef}>
      <TopMenuButton
        icon={faMagnifyingGlass}
        label={search ? `"${search}"` : t('transactions.search.trigger')}
        onClick={() => setIsOpen((open) => !open)}
      />
      {search && <span className="search-button-badge" />}
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover search-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <h4 className="search-popover-title">{t('transactions.search.title')}</h4>
            <input
              type="text"
              className="search-popover-input"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={handleInputKeyDown}
              placeholder={t('transactions.search.placeholder')}
              autoFocus
            />
            <p className="search-popover-help">{t('transactions.search.help')}</p>
            <div className="search-popover-actions">
              <button
                type="button"
                className="search-popover-button"
                disabled={hasNothingToClear}
                onClick={handleClearDraft}
              >
                {t('transactions.search.clear')}
              </button>
              <button type="button" className="search-popover-button" onClick={handleCancel}>
                {t('transactions.search.cancel')}
              </button>
              <button
                type="button"
                className="search-popover-button search-popover-button--primary"
                disabled={hasNothingToClear}
                onClick={handleApply}
              >
                {t('transactions.search.apply')}
              </button>
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
};
