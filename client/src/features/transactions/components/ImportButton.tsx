import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { faFileImport } from '@fortawesome/free-solid-svg-icons';
import { TopMenuButton } from '../../../components/TopMenuButton';

/** Opens the statement import page. */
export const ImportButton = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <TopMenuButton
      icon={faFileImport}
      label={t('transactions.import.trigger')}
      onClick={() => navigate({ to: '/transactions/import' })}
    />
  );
};
