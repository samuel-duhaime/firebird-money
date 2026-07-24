import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import '../../components/Popover.css';
import './SearchButton.css';

const routeApi = getRouteApi('/transactions');

export const SearchButton = () => {
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

  const handleApply = () => {
    if (isDraftEmpty) return;
    applySearch(draft.trim());
    setIsOpen(false);
  };

  const handleClearDraft = () => {
    if (isDraftEmpty) return;
    setDraft('');
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
        label={search ? `"${search}"` : 'Search'}
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
            <h4 className="search-popover-title">Search</h4>
            <input
              type="text"
              className="search-popover-input"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={handleInputKeyDown}
              placeholder="Enter a search term..."
              autoFocus
            />
            <p className="search-popover-help">
              We&apos;ll match your search term to merchant names, categories, and amounts.
            </p>
            <div className="search-popover-actions">
              <button
                type="button"
                className="search-popover-button"
                disabled={isDraftEmpty}
                onClick={handleClearDraft}
              >
                Clear
              </button>
              <button type="button" className="search-popover-button" onClick={handleCancel}>
                Cancel
              </button>
              <button
                type="button"
                className="search-popover-button search-popover-button--primary"
                disabled={isDraftEmpty}
                onClick={handleApply}
              >
                Apply
              </button>
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
};
