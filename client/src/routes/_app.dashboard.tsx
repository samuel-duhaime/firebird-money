import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { useCategories } from '../features/categories/use-categories';

const Dashboard = () => {
  const { t } = useTranslation();
  const { data: categories, isPending, isError } = useCategories();

  return (
    <>
      {isPending && <p>{t('dashboard.loading')}</p>}
      {isError && <p>{t('dashboard.error')}</p>}
      {categories && <p>{t('dashboard.loaded', { count: categories.length })}</p>}
    </>
  );
};

export const Route = createFileRoute('/_app/dashboard')({
  component: Dashboard,
  staticData: { topMenuTitle: 'nav.dashboard' },
});
