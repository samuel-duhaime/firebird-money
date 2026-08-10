import { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { faDownload } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import { downloadFailedToast } from '../../lib/toast';
import { downloadTransactions } from './download';
import type { DownloadFormat } from './download';
import '../../components/Popover.css';
import './DownloadButton.css';

const routeApi = getRouteApi('/_app/transactions');

const DOWNLOAD_FORMATS: DownloadFormat[] = ['csv', 'xlsx'];

export const DownloadButton = () => {
  const { t } = useTranslation();
  const { search, order, start_date, end_date } = routeApi.useSearch();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const firstOptionRef = useRef<HTMLButtonElement>(null);
  const wasOpenRef = useRef(false);

  // Return focus to the trigger once the popover actually closes (selection, outside click, or
  // Escape), but not on first mount.
  useEffect(() => {
    if (isOpen) {
      wasOpenRef.current = true;
    } else if (wasOpenRef.current) {
      wasOpenRef.current = false;
      triggerRef.current?.focus();
    }
  }, [isOpen, triggerRef]);

  // Move focus into the popover once it's mounted and positioned.
  useEffect(() => {
    if (isOpen && position) firstOptionRef.current?.focus();
  }, [isOpen, position]);

  const handleSelect = async (format: DownloadFormat) => {
    setIsOpen(false);
    try {
      await downloadTransactions(format, search, order, start_date, end_date);
    } catch {
      downloadFailedToast();
    }
  };

  return (
    <div className="download-button-trigger">
      <TopMenuButton
        ref={triggerRef}
        icon={faDownload}
        label={t('transactions.download.trigger')}
        aria-haspopup="menu"
        aria-expanded={isOpen}
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
            {DOWNLOAD_FORMATS.map((format, index) => (
              <button
                key={format}
                ref={index === 0 ? firstOptionRef : undefined}
                type="button"
                className="download-popover-option"
                onClick={() => handleSelect(format)}
              >
                {t(`transactions.download.${format}`)}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
};
