import type { SelectHTMLAttributes } from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown } from '@fortawesome/free-solid-svg-icons';
import './Select.css';

type SelectProps = SelectHTMLAttributes<HTMLSelectElement> & {
  /** `field`: a full-width form control, matching the picker triggers (`CategoryPicker`, ...).
   * `cell`: borderless inside a transactions row, with the chevron only on hover/focus — same as
   * the row's picker cells. `compact`: a small toolbar control (e.g. a sort order). */
  variant?: 'field' | 'cell' | 'compact';
  /** Greys the shown text, for a "nothing picked yet" / "No change" value — same as a picker
   * trigger's placeholder. */
  isPlaceholder?: boolean;
};

/**
 * A native `<select>` with the app's own chevron. Use this instead of styling a bare `<select>`:
 * the browser's arrow ignores padding and doesn't match the pickers' chevron.
 */
export const Select = ({
  variant = 'field',
  isPlaceholder = false,
  className,
  children,
  ...props
}: SelectProps) => (
  <span className={`select select--${variant} ${className ?? ''}`}>
    <select
      className={
        isPlaceholder
          ? 'select-input select-input--placeholder'
          : 'select-input'
      }
      {...props}
    >
      {children}
    </select>
    <FontAwesomeIcon
      icon={faChevronDown}
      className="select-chevron"
      aria-hidden="true"
    />
  </span>
);
