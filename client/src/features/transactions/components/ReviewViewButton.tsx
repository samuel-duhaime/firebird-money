import { Fragment } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faCheck,
  faChevronDown,
  faHourglassHalf,
  faListUl,
  faUserClock,
} from '@fortawesome/free-solid-svg-icons';
import type { IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { useAnchoredPopover } from '../../../lib/use-anchored-popover';
import type { ReviewView } from '../utils/filters';
import '../../../components/Popover.css';
import './ReviewViewButton.css';

const VIEW_ICONS: Record<ReviewView, IconDefinition> = {
  all: faListUl,
  needs_review: faHourglassHalf,
  mine: faUserClock,
};

type ReviewViewButtonProps = {
  view: ReviewView;
  /** How many transactions each review view would show; `all` has no count. */
  counts: { needs_review: number; mine: number };
  onChange: (view: ReviewView) => void;
};

/** The "All transactions" dropdown: each option is a full phrase, and the trigger shows that same
 * phrase, so a picked review view still says what it is once the menu closes. */
export const ReviewViewButton = ({
  view,
  counts,
  onChange,
}: ReviewViewButtonProps) => {
  const { t } = useTranslation();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();

  const handleSelect = (value: ReviewView) => {
    onChange(value);
    setIsOpen(false);
  };

  const renderLabel = (value: ReviewView) => (
    <>
      <FontAwesomeIcon icon={VIEW_ICONS[value]} className="review-view-icon" />
      <span>{t(`transactions.review.views.${value}`)}</span>
      {value !== 'all' && (
        <span className="review-view-count">{counts[value]}</span>
      )}
    </>
  );

  return (
    <>
      <button
        type="button"
        className={
          view === 'all'
            ? 'transactions-toolbar-button'
            : 'transactions-toolbar-button review-view-trigger--active'
        }
        ref={triggerRef}
        onClick={() => setIsOpen((open) => !open)}
      >
        {renderLabel(view)}
        <FontAwesomeIcon icon={faChevronDown} />
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover review-view-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            {(['all', 'needs_review', 'mine'] as const).map((value) => (
              <Fragment key={value}>
                {value === 'needs_review' && (
                  <span className="review-view-divider" />
                )}
                <button
                  type="button"
                  className={
                    value === view
                      ? 'review-view-option review-view-option--selected'
                      : 'review-view-option'
                  }
                  onClick={() => handleSelect(value)}
                >
                  {renderLabel(value)}
                </button>
              </Fragment>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
};

/** What a review view shows once nothing is left to review. */
export const ReviewEmptyState = ({ onViewAll }: { onViewAll: () => void }) => {
  const { t } = useTranslation();

  return (
    <div className="review-empty">
      <span className="review-empty-icon" aria-hidden="true">
        <FontAwesomeIcon icon={faCheck} />
      </span>
      <p className="review-empty-title">
        {t('transactions.review.empty.title')}
      </p>
      <p className="review-empty-text">{t('transactions.review.empty.text')}</p>
      <button
        type="button"
        className="transactions-toolbar-button review-empty-button"
        onClick={onViewAll}
      >
        {t('transactions.review.empty.viewAll')}
      </button>
    </div>
  );
};
