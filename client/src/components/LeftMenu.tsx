import { createPortal } from 'react-dom';
import { Link, useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faHouse,
  faReceipt,
  faPeopleRoof,
  faCircleUser,
  faChevronUp,
  faGear,
  faRightFromBracket,
} from '@fortawesome/free-solid-svg-icons';
import { LanguageSwitcher } from './LanguageSwitcher';
import { useAnchoredPopover } from '../lib/use-anchored-popover';
import { useSignOut } from '../features/auth/hooks/use-sign-out';
import { useCurrentUser } from '../features/auth/hooks/use-current-user';
import './Popover.css';
import './LeftMenu.css';

const navItems = [
  { to: '/dashboard', labelKey: 'nav.dashboard', icon: faHouse },
  { to: '/transactions', labelKey: 'nav.transactions', icon: faReceipt },
  { to: '/settings/members', labelKey: 'nav.household', icon: faPeopleRoof },
] as const;

export const LeftMenu = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const signOut = useSignOut();
  const { data: session } = useCurrentUser();
  const { isOpen, setIsOpen, position, triggerRef, popoverRef } =
    useAnchoredPopover<HTMLButtonElement>();

  return (
    <nav className="left-menu">
      <ul className="left-menu-nav">
        {navItems.map(({ to, labelKey, icon }) => (
          <li key={to}>
            <Link
              to={to}
              activeOptions={{ exact: to === '/dashboard' }}
              className="left-menu-link"
              activeProps={{
                className: 'left-menu-link left-menu-link--active',
              }}
            >
              <FontAwesomeIcon icon={icon} className="left-menu-link-icon" />
              <span>{t(labelKey)}</span>
            </Link>
          </li>
        ))}
      </ul>
      <LanguageSwitcher />
      <button
        type="button"
        className="left-menu-profile"
        ref={triggerRef}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((open) => !open)}
      >
        <FontAwesomeIcon
          icon={faCircleUser}
          className="left-menu-profile-icon"
        />
        <span className="left-menu-profile-name">
          {session?.user.first_name ?? session?.user.email}
        </span>
        <FontAwesomeIcon
          icon={faChevronUp}
          className="left-menu-profile-chevron"
        />
      </button>
      {isOpen &&
        position &&
        createPortal(
          <div
            className="anchored-popover left-menu-profile-popover"
            role="menu"
            ref={popoverRef}
            style={{ top: position.top, left: position.left }}
          >
            <button
              type="button"
              role="menuitem"
              className="left-menu-profile-popover-option"
              onClick={() => {
                setIsOpen(false);
                navigate({ to: '/settings/members' });
              }}
            >
              <FontAwesomeIcon icon={faGear} />
              <span>{t('leftMenu.settings')}</span>
            </button>
            <button
              type="button"
              role="menuitem"
              className="left-menu-profile-popover-option left-menu-profile-popover-option--danger"
              disabled={signOut.isPending}
              onClick={() => {
                setIsOpen(false);
                signOut.mutate();
              }}
            >
              <FontAwesomeIcon icon={faRightFromBracket} />
              <span>{t('leftMenu.signOut')}</span>
            </button>
          </div>,
          document.body,
        )}
    </nav>
  );
};
