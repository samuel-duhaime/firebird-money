import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { getRouteApi } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCalendarDays } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../components/TopMenuButton';
import { useAnchoredPopover } from '../../lib/use-anchored-popover';
import {
  DATE_RANGE_PRESETS,
  formatDateRangeParts,
  isValidDateKey,
  resolvePreset,
} from './date-range';
import type { DateRangePreset } from './date-range';
import { toIntlLocale } from '../../i18n/locale';
import '../../components/Popover.css';
import './DateRangeButton.css';

const routeApi = getRouteApi('/transactions');

/**
 * A `YYYY-MM-DD` text field paired with a calendar icon that opens a native date picker. The
 * native `<input type="date">` is kept invisible (its own display format is locale-dependent) and
 * only used to drive the picker UI — its `value` is always `YYYY-MM-DD` regardless of locale, so
 * picking a date there just fills in the visible text field.
 */
const DateField = ({
  id,
  label,
  value,
  isValid,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  isValid: boolean;
  onChange: (value: string) => void;
}) => {
  const { t } = useTranslation();
  const pickerRef = useRef<HTMLInputElement>(null);

  const openPicker = () => {
    const picker = pickerRef.current;
    if (!picker) return;
    if (typeof picker.showPicker === 'function') picker.showPicker();
    else picker.focus();
  };

  return (
    <div className="date-range-popover-field">
      <div className="date-range-popover-field-header">
        <label htmlFor={id}>{label}</label>
        <button
          type="button"
          className="date-range-popover-field-clear"
          disabled={!value}
          onClick={() => onChange('')}
        >
          {t('transactions.dateRange.clear')}
        </button>
      </div>
      <div className="date-range-popover-input-wrapper">
        <input
          id={id}
          type="text"
          placeholder={t('transactions.dateRange.placeholder')}
          maxLength={10}
          value={value}
          aria-invalid={!isValid}
          onChange={(event) => onChange(event.target.value)}
        />
        <input
          ref={pickerRef}
          type="date"
          className="date-range-popover-hidden-picker"
          value={isValid && value ? value : ''}
          onChange={(event) => onChange(event.target.value)}
          tabIndex={-1}
          aria-hidden="true"
        />
        <button
          type="button"
          className="date-range-popover-calendar-button"
          aria-label={t('transactions.dateRange.pickA', { label: label.toLowerCase() })}
          onClick={openPicker}
        >
          <FontAwesomeIcon icon={faCalendarDays} />
        </button>
      </div>
    </div>
  );
};

export const DateRangeButton = () => {
  const { t, i18n } = useTranslation();
  const locale = toIntlLocale(i18n.resolvedLanguage ?? i18n.language);
  const { start_date, end_date } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();
  const firstOptionRef = useRef<HTMLButtonElement>(null);
  const wasOpenRef = useRef(false);

  const [draftStart, setDraftStart] = useState(start_date ?? '');
  const [draftEnd, setDraftEnd] = useState(end_date ?? '');

  useEffect(() => {
    if (isOpen) {
      setDraftStart(start_date ?? '');
      setDraftEnd(end_date ?? '');
    }
  }, [isOpen, start_date, end_date]);

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

  const hasActiveRange = Boolean(start_date || end_date);
  const isDraftEmpty = !draftStart && !draftEnd;
  const hasNothingToClear = isDraftEmpty && !hasActiveRange;

  const isDraftStartValid = draftStart === '' || isValidDateKey(draftStart);
  const isDraftEndValid = draftEnd === '' || isValidDateKey(draftEnd);
  const isRangeInvalid =
    !isDraftStartValid ||
    !isDraftEndValid ||
    Boolean(
      draftStart &&
      draftEnd &&
      isDraftStartValid &&
      isDraftEndValid &&
      draftStart > draftEnd,
    );

  const applyRange = (range: { start_date?: string; end_date?: string }) => {
    navigate({ search: (prev) => ({ ...prev, ...range }), replace: true });
  };

  const handlePreset = (preset: DateRangePreset) => {
    applyRange(resolvePreset(preset));
    setIsOpen(false);
  };

  const handleApply = () => {
    if (isRangeInvalid) return;
    applyRange({
      start_date: draftStart || undefined,
      end_date: draftEnd || undefined,
    });
    setIsOpen(false);
  };

  const handleClearAll = () => {
    if (hasNothingToClear) return;
    applyRange({ start_date: undefined, end_date: undefined });
    setIsOpen(false);
  };

  const handleCancel = () => {
    setDraftStart(start_date ?? '');
    setDraftEnd(end_date ?? '');
    setIsOpen(false);
  };

  const rangeParts = formatDateRangeParts(start_date, end_date, locale);
  const rangeLabel =
    rangeParts.kind === 'none'
      ? t('transactions.dateRange.trigger')
      : rangeParts.kind === 'single'
        ? rangeParts.date
        : rangeParts.kind === 'range'
          ? `${rangeParts.start} – ${rangeParts.end}`
          : rangeParts.kind === 'from'
            ? t('transactions.dateRange.from', { date: rangeParts.date })
            : t('transactions.dateRange.until', { date: rangeParts.date });

  return (
    <div className="date-range-button-trigger">
      <TopMenuButton
        ref={triggerRef}
        icon={faCalendarDays}
        label={rangeLabel}
        aria-haspopup="dialog"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((open) => !open)}
      />
      {hasActiveRange && <span className="date-range-button-badge" />}
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover date-range-popover"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <div className="date-range-popover-columns">
              <div className="date-range-popover-presets">
                <h4 className="date-range-popover-title">{t('transactions.dateRange.title')}</h4>
                <div className="date-range-popover-preset-list">
                  {DATE_RANGE_PRESETS.map((preset, index) => (
                    <button
                      key={preset}
                      ref={index === 0 ? firstOptionRef : undefined}
                      type="button"
                      className="date-range-popover-preset"
                      onClick={() => handlePreset(preset)}
                    >
                      {t(`transactions.dateRange.presets.${preset}`)}
                    </button>
                  ))}
                </div>
                <button
                  type="button"
                  className="date-range-popover-clear-all"
                  disabled={hasNothingToClear}
                  onClick={handleClearAll}
                >
                  {t('transactions.dateRange.clear')}
                </button>
              </div>
              <div className="date-range-popover-fields">
                <DateField
                  id="date-range-start-date"
                  label={t('transactions.dateRange.startDate')}
                  value={draftStart}
                  isValid={isDraftStartValid}
                  onChange={setDraftStart}
                />
                <DateField
                  id="date-range-end-date"
                  label={t('transactions.dateRange.endDate')}
                  value={draftEnd}
                  isValid={isDraftEndValid}
                  onChange={setDraftEnd}
                />
              </div>
            </div>
            <div className="date-range-popover-actions">
              <button
                type="button"
                className="date-range-popover-button"
                onClick={handleCancel}
              >
                {t('transactions.dateRange.cancel')}
              </button>
              <button
                type="button"
                className="date-range-popover-button date-range-popover-button--primary"
                disabled={isRangeInvalid}
                onClick={handleApply}
              >
                {t('transactions.dateRange.apply')}
              </button>
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
};
