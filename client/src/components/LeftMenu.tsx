import { Link } from '@tanstack/react-router';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faHouse, faReceipt, faRuler, faCircleUser } from '@fortawesome/free-solid-svg-icons';
import './LeftMenu.css';

const navItems = [
  { to: '/', label: 'Dashboard', icon: faHouse },
  { to: '/transactions', label: 'Transactions', icon: faReceipt },
  { to: '/rules', label: 'Rules', icon: faRuler },
];

export const LeftMenu = () => (
  <nav className="left-menu">
    <ul className="left-menu-nav">
      {navItems.map(({ to, label, icon }) => (
        <li key={to}>
          <Link
            to={to}
            activeOptions={{ exact: to === '/' }}
            className="left-menu-link"
            activeProps={{ className: 'left-menu-link left-menu-link--active' }}
          >
            <FontAwesomeIcon icon={icon} className="left-menu-link-icon" />
            <span>{label}</span>
          </Link>
        </li>
      ))}
    </ul>
    <div className="left-menu-profile">
      <FontAwesomeIcon icon={faCircleUser} className="left-menu-profile-icon" />
      <span className="left-menu-profile-name">Username</span>
    </div>
  </nav>
);
