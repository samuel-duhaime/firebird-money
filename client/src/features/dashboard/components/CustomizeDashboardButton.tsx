import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faAngleDown,
  faAngleUp,
  faGripVertical,
  faTableCellsLarge,
  faXmark,
} from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../../components/TopMenuButton';
import { Switch } from '../../../components/Switch';
import { useDashboardWidgets } from '../hooks/use-dashboard-widgets';
import { useSaveDashboardWidgets } from '../hooks/use-save-dashboard-widgets';
import { useDragReorder } from '../../../lib/use-drag-reorder';
import type {
  DashboardWidget,
  DashboardWidgetId,
} from '../../settings/utils/types';
import '../../../components/MoveButtons.css';
import '../../transactions/components/AddTransactionModal.css';
import './CustomizeDashboardButton.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

/** Picks which widgets the dashboard shows, and in what order. Every change saves right away. */
const CustomizeDashboardModal = ({ onClose }: { onClose: () => void }) => {
  const { t } = useTranslation();
  const { save, reorderMutation } = useSaveDashboardWidgets();
  const widgets = useDashboardWidgets();
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);

  const {
    items,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    moveUp,
    moveDown,
  } = useDragReorder<DashboardWidget, DashboardWidgetId>(
    widgets,
    (widget) => widget.id,
    reorderMutation(widgets),
    // `save` already shows the toast.
    () => {},
  );

  const toggle = (id: DashboardWidgetId, visible: boolean) =>
    save(
      items.map((widget) =>
        widget.id === id ? { ...widget, visible } : widget,
      ),
    );

  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      onClose();
      return;
    }
    if (event.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    ).filter(
      (element) => element.tabIndex !== -1 && !element.matches(':disabled'),
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return createPortal(
    <div
      className="modal-overlay"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="customize-dashboard"
        role="dialog"
        aria-modal="true"
        aria-labelledby="customize-dashboard-title"
        aria-describedby="customize-dashboard-description"
        onKeyDown={handleKeyDown}
      >
        <div className="customize-dashboard-header">
          <div>
            <h2
              id="customize-dashboard-title"
              className="customize-dashboard-title"
            >
              {t('dashboard.customize.title')}
            </h2>
            <p
              id="customize-dashboard-description"
              className="customize-dashboard-description"
            >
              {t('dashboard.customize.description')}
            </p>
          </div>
          <button
            ref={closeRef}
            type="button"
            className="customize-dashboard-close"
            onClick={onClose}
            aria-label={t('dashboard.customize.close')}
          >
            <FontAwesomeIcon icon={faXmark} />
          </button>
        </div>
        <ul className="customize-dashboard-list">
          {items.map((widget, index) => {
            const name = t(`dashboard.widgets.${widget.id}`);
            return (
              <li
                key={widget.id}
                className="customize-dashboard-row"
                onDragOver={handleDragOver(index)}
                onDrop={handleDrop}
              >
                <span
                  className="customize-dashboard-drag-handle"
                  draggable
                  onDragStart={handleDragStart(index)}
                  onDragEnd={handleDragEnd}
                  aria-hidden="true"
                >
                  <FontAwesomeIcon icon={faGripVertical} />
                </span>
                <div className="move-buttons">
                  <button
                    type="button"
                    className="move-button"
                    onClick={() => moveUp(index)}
                    disabled={index === 0}
                    aria-label={t('dashboard.customize.moveUp', { name })}
                  >
                    <FontAwesomeIcon icon={faAngleUp} />
                  </button>
                  <button
                    type="button"
                    className="move-button"
                    onClick={() => moveDown(index)}
                    disabled={index === items.length - 1}
                    aria-label={t('dashboard.customize.moveDown', { name })}
                  >
                    <FontAwesomeIcon icon={faAngleDown} />
                  </button>
                </div>
                <span className="customize-dashboard-name">{name}</span>
                <Switch
                  checked={widget.visible}
                  onChange={(visible) => toggle(widget.id, visible)}
                  ariaLabel={t('dashboard.customize.show', { name })}
                />
              </li>
            );
          })}
        </ul>
      </div>
    </div>,
    document.body,
  );
};

/** The dashboard's top menu "Customize" button, and the modal it opens. */
export const CustomizeDashboardButton = () => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const close = () => {
    setIsOpen(false);
    buttonRef.current?.focus();
  };

  return (
    <>
      <TopMenuButton
        ref={buttonRef}
        icon={faTableCellsLarge}
        label={t('dashboard.customize.button')}
        aria-haspopup="dialog"
        aria-expanded={isOpen}
        onClick={() => setIsOpen(true)}
      />
      {isOpen && <CustomizeDashboardModal onClose={close} />}
    </>
  );
};
