import type { IconDefinition } from '@fortawesome/fontawesome-svg-core';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import './TopMenuButton.css';

type TopMenuButtonProps = {
  icon: IconDefinition;
  label: string;
  variant?: 'default' | 'primary';
  onClick?: () => void;
};

export const TopMenuButton = ({ icon, label, variant = 'default', onClick }: TopMenuButtonProps) => (
  <button
    type="button"
    className={variant === 'primary' ? 'top-menu-button top-menu-button--primary' : 'top-menu-button'}
    onClick={onClick}
  >
    <FontAwesomeIcon icon={icon} />
    <span>{label}</span>
  </button>
);
