import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import './SearchButton.css';

const routeApi = getRouteApi('/transactions');

type Position = { top: number; left: number };

export const SearchButton = () => {
  const { search } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const [isOpen, setIsOpen] = useState(false);
  const [draft, setDraft] = useState(search ?? '');
  const [position, setPosition] = useState<Position | null>(null);
  const triggerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isOpen) setDraft(search ?? '');
  }, [isOpen, search]);

  useEffect(() => {
    if (!isOpen) return;

    const updatePosition = () => {
      const rect = triggerRef.current?.getBoundingClientRect();
      if (rect) setPosition({ top: rect.bottom + 8, left: rect.left });
    };
    updatePosition();

    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (triggerRef.current?.contains(target) || popoverRef.current?.contains(target)) return;
      setIsOpen(false);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsOpen(false);
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

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

  const handleClearApplied = () => applySearch(undefined);

  const handleInputKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') handleApply();
  };

  return (
    <div className="search-button">
      {search && (
        <button type="button" className="search-button-clear-applied" onClick={handleClearApplied}>
          Clear
        </button>
      )}
      <div className="search-button-trigger" ref={triggerRef}>
        <TopMenuButton
          icon={faMagnifyingGlass}
          label={search ? `"${search}"` : 'Search'}
          onClick={() => setIsOpen((open) => !open)}
        />
        {search && <span className="search-button-badge" />}
      </div>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="search-popover"
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
