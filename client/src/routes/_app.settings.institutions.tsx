import { useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { AccountModal } from '../features/accounts/components/AccountModal';
import { InstitutionsList } from '../features/institutions/components/InstitutionsList';
import { InstitutionModal } from '../features/institutions/components/InstitutionModal';
import type { Account } from '../features/accounts/utils/types';
import type { Institution } from '../features/institutions/utils/types';
import './_app.settings.css';

type ModalState =
  | { kind: 'addAccount'; institutionId?: number }
  | { kind: 'editAccount'; account: Account }
  | { kind: 'editInstitution'; institution: Institution }
  | null;

const InstitutionsPage = () => {
  const { t } = useTranslation();
  const [modal, setModal] = useState<ModalState>(null);
  const closeModal = () => setModal(null);

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.institutions.heading')}</h2>
        <button
          type="button"
          className="settings-panel-primary-button"
          onClick={() => setModal({ kind: 'addAccount' })}
        >
          {t('settings.institutions.addAccount')}
        </button>
      </div>
      <p className="settings-panel-description">
        {t('settings.institutions.description')}
      </p>

      <InstitutionsList
        onAddAccount={(institutionId) =>
          setModal({ kind: 'addAccount', institutionId })
        }
        onEditAccount={(account) => setModal({ kind: 'editAccount', account })}
        onEditInstitution={(institution) =>
          setModal({ kind: 'editInstitution', institution })
        }
      />

      {modal?.kind === 'addAccount' && (
        <AccountModal
          institutionId={modal.institutionId}
          onClose={closeModal}
        />
      )}
      {modal?.kind === 'editAccount' && (
        <AccountModal account={modal.account} onClose={closeModal} />
      )}
      {modal?.kind === 'editInstitution' && (
        <InstitutionModal
          institution={modal.institution}
          onClose={closeModal}
        />
      )}
    </div>
  );
};

export const Route = createFileRoute('/_app/settings/institutions')({
  component: InstitutionsPage,
});
