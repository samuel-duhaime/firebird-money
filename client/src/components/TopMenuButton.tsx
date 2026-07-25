import type { AriaAttributes } from 'react';
import type { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import { forwardRef } from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import './TopMenuButton.css';

type TopMenuButtonProps = {
  icon: IconDefinition;
  label: string;
  variant?: 'default' | 'primary';
  onClick?: () => void;
  'aria-haspopup'?: AriaAttributes['aria-haspopup'];
  'aria-expanded'?: boolean;
};

export const TopMenuButton = forwardRef<HTMLButtonElement, TopMenuButtonProps>(
  (
    {
      icon,
      label,
      variant = 'default',
      onClick,
      'aria-haspopup': ariaHasPopup,
      'aria-expanded': ariaExpanded,
    },
    ref,
  ) => (
    <button
      ref={ref}
      type="button"
      className={
        variant === 'primary'
          ? 'top-menu-button top-menu-button--primary'
          : 'top-menu-button'
      }
      onClick={onClick}
      aria-haspopup={ariaHasPopup}
      aria-expanded={ariaExpanded}
    >
      <FontAwesomeIcon icon={icon} />
      <span>{label}</span>
    </button>
  ),
);

TopMenuButton.displayName = 'TopMenuButton';
