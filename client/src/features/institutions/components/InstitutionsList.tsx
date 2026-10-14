import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ApiError } from '../../../lib/api-client';
import {
  deleteAccountFailedToast,
  deleteAccountInUseToast,
  deleteInstitutionFailedToast,
} from '../../../lib/toast';
import { useInstitutions } from '../hooks/use-institutions';
import { useDeleteInstitution } from '../hooks/use-delete-institution';
import type { Institution } from '../utils/types';
import { useAccounts } from '../../accounts/hooks/use-accounts';
import { useDeleteAccount } from '../../accounts/hooks/use-delete-account';
import { accountLabel } from '../../accounts/utils/format';
import type { Account } from '../../accounts/utils/types';
import { InstitutionAvatar } from './InstitutionAvatar';
import './InstitutionsList.css';

type InstitutionGroup = {
  institution: Institution;
  accounts: Account[];
};

type InstitutionsListProps = {
  onAddAccount: (institutionId: number) => void;
  onEditAccount: (account: Account) => void;
  onEditInstitution: (institution: Institution) => void;
};

/** What's waiting for an inline delete confirmation, if anything. */
type ConfirmingDelete =
  { kind: 'account'; id: number } | { kind: 'institution'; id: number } | null;

/**
 * One card per institution the household uses, with its accounts listed underneath. A custom
 * institution shows up even with no accounts left, so it can still be renamed or deleted; a common
 * one only shows while the household has an account there.
 */
export const InstitutionsList = ({
  onAddAccount,
  onEditAccount,
  onEditInstitution,
}: InstitutionsListProps) => {
  const { t } = useTranslation();
  const accountsQuery = useAccounts();
  const institutionsQuery = useInstitutions();
  const deleteAccountMutation = useDeleteAccount();
  const deleteInstitutionMutation = useDeleteInstitution();
  const [confirmingDelete, setConfirmingDelete] =
    useState<ConfirmingDelete>(null);

  const isPending = accountsQuery.isPending || institutionsQuery.isPending;
  const isError = accountsQuery.isError || institutionsQuery.isError;

  const groups = useMemo<InstitutionGroup[]>(() => {
    const accounts = accountsQuery.data ?? [];
    const institutions = institutionsQuery.data ?? [];
    // `institutions` already arrives A-Z, so grouping in its order keeps the cards sorted.
    return institutions
      .map((institution) => ({
        institution,
        accounts: accounts.filter((a) => a.institution_id === institution.id),
      }))
      .filter(
        (group) =>
          group.accounts.length > 0 || group.institution.household_id !== null,
      );
  }, [accountsQuery.data, institutionsQuery.data]);

  const handleDeleteAccount = (id: number) => {
    deleteAccountMutation.mutate(id, {
      onSuccess: () => setConfirmingDelete(null),
      onError: (error) => {
        // 409 is specifically "still used by existing transactions" (account-in-use).
        if (error instanceof ApiError && error.status === 409) {
          deleteAccountInUseToast();
        } else {
          deleteAccountFailedToast();
        }
      },
    });
  };

  const handleDeleteInstitution = (id: number) => {
    deleteInstitutionMutation.mutate(id, {
      onSuccess: () => setConfirmingDelete(null),
      onError: () => deleteInstitutionFailedToast(),
    });
  };

  if (isPending) return <p>{t('settings.institutions.loading')}</p>;
  if (isError) return <p>{t('settings.institutions.error')}</p>;
  if (groups.length === 0) {
    return (
      <p className="institutions-list-empty">
        {t('settings.institutions.empty')}
      </p>
    );
  }

  return (
    <div className="institutions-list">
      {groups.map(({ institution, accounts }) => {
        const isCustom = institution.household_id !== null;
        const isConfirmingInstitution =
          confirmingDelete?.kind === 'institution' &&
          confirmingDelete.id === institution.id;

        return (
          <section
            key={institution.id}
            className="institutions-list-card"
            aria-label={institution.name}
          >
            <header className="institutions-list-card-header">
              <InstitutionAvatar name={institution.name} />
              <div className="institutions-list-card-title">
                <span className="institutions-list-institution-name">
                  {institution.name}
                </span>
                <span className="institutions-list-meta">
                  {t('settings.institutions.accountCount', {
                    count: accounts.length,
                  })}
                </span>
              </div>

              {isConfirmingInstitution ? (
                <div className="institutions-list-confirm">
                  <span className="institutions-list-confirm-text">
                    {t('settings.institutions.deleteInstitutionConfirm', {
                      name: institution.name,
                    })}
                  </span>
                  <div className="institutions-list-actions">
                    <button
                      type="button"
                      className="institutions-list-action"
                      onClick={() => setConfirmingDelete(null)}
                    >
                      {t('settings.institutions.modal.cancel')}
                    </button>
                    <button
                      type="button"
                      className="institutions-list-action institutions-list-action--danger"
                      onClick={() => handleDeleteInstitution(institution.id)}
                      disabled={deleteInstitutionMutation.isPending}
                    >
                      {t('settings.institutions.deleteConfirmButton')}
                    </button>
                  </div>
                </div>
              ) : (
                <div className="institutions-list-actions">
                  {isCustom && (
                    <button
                      type="button"
                      className="institutions-list-action"
                      onClick={() => onEditInstitution(institution)}
                    >
                      {t('settings.institutions.edit')}
                    </button>
                  )}
                  {/* The server refuses to delete an institution that still has accounts, so
                      the button only appears once it's empty. */}
                  {isCustom && accounts.length === 0 && (
                    <button
                      type="button"
                      className="institutions-list-action institutions-list-action--danger"
                      onClick={() =>
                        setConfirmingDelete({
                          kind: 'institution',
                          id: institution.id,
                        })
                      }
                    >
                      {t('settings.institutions.delete')}
                    </button>
                  )}
                  <button
                    type="button"
                    className="institutions-list-action"
                    onClick={() => onAddAccount(institution.id)}
                  >
                    {t('settings.institutions.addHere')}
                  </button>
                </div>
              )}
            </header>

            {accounts.length > 0 && (
              <ul className="institutions-list-rows">
                {accounts.map((account) => {
                  const isConfirming =
                    confirmingDelete?.kind === 'account' &&
                    confirmingDelete.id === account.id;

                  return (
                    <li key={account.id} className="institutions-list-row">
                      <div className="institutions-list-account">
                        <span className="institutions-list-account-name">
                          {accountLabel(account)}
                        </span>
                        <span className="institutions-list-meta">
                          {[
                            t(`settings.institutions.types.${account.type}`),
                            account.currency,
                            t('settings.institutions.transactionCount', {
                              count: account.transaction_count,
                            }),
                          ].join(' · ')}
                        </span>
                      </div>

                      {isConfirming ? (
                        <div className="institutions-list-confirm">
                          <span className="institutions-list-confirm-text">
                            {t('settings.institutions.deleteAccountConfirm', {
                              name: account.name,
                            })}
                          </span>
                          <div className="institutions-list-actions">
                            <button
                              type="button"
                              className="institutions-list-action"
                              onClick={() => setConfirmingDelete(null)}
                            >
                              {t('settings.institutions.modal.cancel')}
                            </button>
                            <button
                              type="button"
                              className="institutions-list-action institutions-list-action--danger"
                              onClick={() => handleDeleteAccount(account.id)}
                              disabled={deleteAccountMutation.isPending}
                            >
                              {t('settings.institutions.deleteConfirmButton')}
                            </button>
                          </div>
                        </div>
                      ) : (
                        <div className="institutions-list-actions">
                          <button
                            type="button"
                            className="institutions-list-action"
                            onClick={() => onEditAccount(account)}
                          >
                            {t('settings.institutions.edit')}
                          </button>
                          <button
                            type="button"
                            className="institutions-list-action institutions-list-action--danger"
                            onClick={() =>
                              setConfirmingDelete({
                                kind: 'account',
                                id: account.id,
                              })
                            }
                          >
                            {t('settings.institutions.delete')}
                          </button>
                        </div>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        );
      })}
    </div>
  );
};
