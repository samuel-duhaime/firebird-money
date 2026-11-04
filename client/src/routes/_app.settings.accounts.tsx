import { useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { AccountModal } from '../features/accounts/components/AccountModal';
import { AccountOwnersModal } from '../features/accounts/components/AccountOwnersModal';
import { useAccounts } from '../features/accounts/hooks/use-accounts';
import { InstitutionsList } from '../features/institutions/components/InstitutionsList';
import { InstitutionModal } from '../features/institutions/components/InstitutionModal';
import type { Account } from '../features/accounts/utils/types';
import type { Institution } from '../features/institutions/utils/types';
import './_app.settings.css';

type ModalState =
  | { kind: 'addAccount'; institutionId?: number }
  | { kind: 'editAccount'; account: Account }
  | { kind: 'editInstitution'; institution: Institution }
  | { kind: 'editOwners' }
  | null;

const AccountsPage = () => {
  const { t } = useTranslation();
  const [modal, setModal] = useState<ModalState>(null);
  const closeModal = () => setModal(null);
  const { data: accounts } = useAccounts();

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.institutions.heading')}</h2>
        <div className="settings-panel-header-actions">
          {accounts && accounts.length > 0 && (
            <button
              type="button"
              className="settings-panel-secondary-button"
              onClick={() => setModal({ kind: 'editOwners' })}
            >
              {t('settings.institutions.editOwners')}
            </button>
          )}
          <button
            type="button"
            className="settings-panel-primary-button"
            onClick={() => setModal({ kind: 'addAccount' })}
          >
            {t('settings.institutions.addAccount')}
          </button>
        </div>
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
      {modal?.kind === 'editOwners' && (
        <AccountOwnersModal onClose={closeModal} />
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

export const Route = createFileRoute('/_app/settings/accounts')({
  component: AccountsPage,
});
