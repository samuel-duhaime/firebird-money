import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { faDownload } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import { downloadFailedToast } from '../../lib/toast';
import { downloadTransactions } from './download';
import type { DownloadFormat } from './download';
import '../../components/Popover.css';
import './DownloadButton.css';

const routeApi = getRouteApi('/transactions');

const DOWNLOAD_OPTIONS: { value: DownloadFormat; label: string }[] = [
  { value: 'csv', label: 'Download as CSV' },
  { value: 'xlsx', label: 'Download as Excel' },
];

export const DownloadButton = () => {
  const { search, order } = routeApi.useSearch();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover();

  const handleSelect = async (format: DownloadFormat) => {
    setIsOpen(false);
    try {
      await downloadTransactions(format, search, order);
    } catch {
      downloadFailedToast();
    }
  };

  return (
    <div className="download-button-trigger" ref={triggerRef}>
      <TopMenuButton
        icon={faDownload}
        label="Download"
        onClick={() => setIsOpen((open) => !open)}
      />
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover download-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            {DOWNLOAD_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                className="download-popover-option"
                onClick={() => handleSelect(option.value)}
              >
                {option.label}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
};
