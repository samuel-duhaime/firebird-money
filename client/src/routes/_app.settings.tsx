import { createFileRoute, Link, Outlet } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { notImplementedToast } from '../lib/toast';
import './_app.settings.css';

type HouseholdNavItem =
  | { key: string; labelKey: string }
  | {
      key: string;
      labelKey: string;
      to: '/settings/tags' | '/settings/categories' | '/settings/merchants';
    };

const HOUSEHOLD_NAV_ITEMS: HouseholdNavItem[] = [
  { key: 'members', labelKey: 'settings.household.members' },
  { key: 'preferences', labelKey: 'settings.household.preferences' },
  { key: 'institutions', labelKey: 'settings.household.institutions' },
  { key: 'accounts', labelKey: 'settings.household.accounts' },
  {
    key: 'categories',
    labelKey: 'settings.household.categories',
    to: '/settings/categories',
  },
  {
    key: 'merchants',
    labelKey: 'settings.household.merchants',
    to: '/settings/merchants',
  },
  { key: 'rules', labelKey: 'settings.household.rules' },
  { key: 'tags', labelKey: 'settings.household.tags', to: '/settings/tags' },
  { key: 'data', labelKey: 'settings.household.data' },
];

const SettingsLayout = () => {
  const { t } = useTranslation();

  return (
    <div className="settings-layout">
      <nav className="settings-nav">
        <p className="settings-nav-heading">{t('settings.householdHeading')}</p>
        <ul className="settings-nav-list">
          {HOUSEHOLD_NAV_ITEMS.map((item) =>
            'to' in item ? (
              <li key={item.key}>
                <Link
                  to={item.to}
                  className="settings-nav-link"
                  activeProps={{
                    className: 'settings-nav-link settings-nav-link--active',
                  }}
                >
                  {t(item.labelKey)}
                </Link>
              </li>
            ) : (
              <li key={item.key}>
                <button
                  type="button"
                  className="settings-nav-link"
                  onClick={notImplementedToast}
                >
                  {t(item.labelKey)}
                </button>
              </li>
            ),
          )}
        </ul>
      </nav>
      <div className="settings-content">
        <Outlet />
      </div>
    </div>
  );
};

export const Route = createFileRoute('/_app/settings')({
  component: SettingsLayout,
  staticData: { topMenuTitle: 'settings.title' },
});
