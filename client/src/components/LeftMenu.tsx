import { Link } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faHouse, faReceipt, faRuler, faCircleUser } from '@fortawesome/free-solid-svg-icons';
import { LanguageSwitcher } from './LanguageSwitcher';
import './LeftMenu.css';

const navItems = [
  { to: '/dashboard', labelKey: 'nav.dashboard', icon: faHouse },
  { to: '/transactions', labelKey: 'nav.transactions', icon: faReceipt },
  { to: '/rules', labelKey: 'nav.rules', icon: faRuler },
] as const;

export const LeftMenu = () => {
  const { t } = useTranslation();

  return (
    <nav className="left-menu">
      <ul className="left-menu-nav">
        {navItems.map(({ to, labelKey, icon }) => (
          <li key={to}>
            <Link
              to={to}
              activeOptions={{ exact: to === '/dashboard' }}
              className="left-menu-link"
              activeProps={{ className: 'left-menu-link left-menu-link--active' }}
            >
              <FontAwesomeIcon icon={icon} className="left-menu-link-icon" />
              <span>{t(labelKey)}</span>
            </Link>
          </li>
        ))}
      </ul>
      <LanguageSwitcher />
      <div className="left-menu-profile">
        <FontAwesomeIcon icon={faCircleUser} className="left-menu-profile-icon" />
        <span className="left-menu-profile-name">{t('leftMenu.username')}</span>
      </div>
    </nav>
  );
};
