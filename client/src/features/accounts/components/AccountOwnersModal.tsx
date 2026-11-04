import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCheck, faLink } from '@fortawesome/free-solid-svg-icons';
import { Avatar } from '../../../components/Avatar';
import { Tooltip } from '../../../components/Tooltip';
import {
  updateAccountOwnersFailedToast,
  updateAccountOwnersSucceededToast,
} from '../../../lib/toast';
import { InstitutionAvatar } from '../../institutions/components/InstitutionAvatar';
import { useMembers } from '../../members/hooks/use-members';
import { memberDisplayName, pictureSrc } from '../../members/utils/format';
import { useAccounts } from '../hooks/use-accounts';
import { useUpdateAccountOwners } from '../hooks/use-update-account-owners';
import { toOwner } from '../utils/api';
import { accountLabel } from '../utils/format';
import { ACCOUNT_TYPES } from '../utils/types';
import type { Account } from '../utils/types';
import '../../transactions/components/AddTransactionModal.css';
import './AccountModal.css';
import './AccountOwnersModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

type AccountOwnersModalProps = {
  onClose: () => void;
};

/** One choice in an account's row: a member's avatar, or the Shared icon. */
type OwnerChoice = { owner: number | null; label: string; tooltip: string };

/**
 * Sets the owner of many accounts at once: one row per account, grouped by type, with a button
 * for each member plus Shared. Nothing is saved until Save, and then all of it or none.
 */
export const AccountOwnersModal = ({ onClose }: AccountOwnersModalProps) => {
  const { t } = useTranslation();
  const { data: accounts } = useAccounts();
  const { data: members } = useMembers();
  const updateOwnersMutation = useUpdateAccountOwners();
  // The owners picked so far, by account id; an account missing here keeps its own.
  const [draft, setDraft] = useState<Map<number, number | null>>(new Map());
  const dialogRef = useRef<HTMLDivElement>(null);
  const hasFocusedRef = useRef(false);

  // Into the dialog right away, so Escape and Tab work there, then onto the first account's
  // owner once the accounts and members have loaded. Only once: never pulls focus back later.
  useEffect(() => {
    if (hasFocusedRef.current) return;
    const chosen = dialogRef.current?.querySelector<HTMLElement>(
      '[role="radio"][aria-checked="true"]',
    );
    if (chosen) {
      chosen.focus();
      hasFocusedRef.current = true;
    } else if (!dialogRef.current?.contains(document.activeElement)) {
      dialogRef.current?.focus();
    }
  }, [accounts, members]);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
        [],
    );
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  const ownerOf = (account: Account) =>
    draft.has(account.id) ? draft.get(account.id)! : account.owner_user_id;

  const choose = (account: Account, owner: number | null) => {
    setDraft((previous) => new Map(previous).set(account.id, owner));
  };

  const choices: OwnerChoice[] = [
    ...(members ?? []).map((member) => {
      const name = memberDisplayName(member);
      return {
        owner: member.id,
        label: name,
        tooltip: name === member.email ? name : `${name}\n${member.email}`,
      };
    }),
    {
      owner: null,
      label: t('settings.members.owner.shared'),
      tooltip: t('settings.members.owner.shared'),
    },
  ];

  /** Arrows move the choice within a row, like a radio group. */
  const handleRowKeyDown = (
    e: ReactKeyboardEvent<HTMLDivElement>,
    account: Account,
  ) => {
    const step =
      e.key === 'ArrowRight' || e.key === 'ArrowDown'
        ? 1
        : e.key === 'ArrowLeft' || e.key === 'ArrowUp'
          ? -1
          : 0;
    if (step === 0) return;
    e.preventDefault();
    const index = choices.findIndex((c) => c.owner === ownerOf(account));
    const next = choices[(index + step + choices.length) % choices.length];
    choose(account, next.owner);
    const buttons =
      e.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]');
    buttons[choices.indexOf(next)]?.focus();
  };

  const handleSave = () => {
    const owners = (accounts ?? [])
      .filter((account) => ownerOf(account) !== account.owner_user_id)
      .map((account) => ({
        account_id: account.id,
        owner: toOwner(ownerOf(account)),
      }));
    if (owners.length === 0) {
      onClose();
      return;
    }
    updateOwnersMutation.mutate(owners, {
      onSuccess: () => {
        updateAccountOwnersSucceededToast();
        onClose();
      },
      onError: () => updateAccountOwnersFailedToast(),
    });
  };

  const groups = ACCOUNT_TYPES.map((type) => ({
    type,
    accounts: (accounts ?? []).filter((account) => account.type === type),
  })).filter((group) => group.accounts.length > 0);

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="account-modal account-owners-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="account-owners-title"
        aria-describedby="account-owners-description"
        tabIndex={-1}
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="account-owners-title">
            {t('settings.institutions.owners.title')}
          </h2>
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('settings.institutions.owners.close')}
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <p id="account-owners-description" className="account-modal-hint">
            {t('settings.institutions.owners.description')}
          </p>

          {groups.map(({ type, accounts: groupAccounts }) => (
            <section
              key={type}
              className="account-owners-group"
              aria-label={t(`settings.institutions.types.${type}`)}
            >
              <div className="account-owners-group-header">
                <span>{t(`settings.institutions.types.${type}`)}</span>
                <span>{t('settings.institutions.owners.owner')}</span>
              </div>
              <ul className="account-owners-rows">
                {groupAccounts.map((account) => {
                  const owner = ownerOf(account);
                  return (
                    <li key={account.id} className="account-owners-row">
                      <InstitutionAvatar name={account.institution_name} />
                      <span className="account-owners-account">
                        <span className="account-owners-account-name">
                          {accountLabel(account)}
                        </span>
                        <span className="account-owners-account-meta">
                          {account.institution_name}
                        </span>
                      </span>
                      <div
                        className="account-owners-choices"
                        role="radiogroup"
                        aria-label={`${t('settings.institutions.owners.owner')}: ${accountLabel(account)}`}
                        onKeyDown={(e) => handleRowKeyDown(e, account)}
                      >
                        {choices.map((choice) => {
                          const isChosen = choice.owner === owner;
                          const member = members?.find(
                            (m) => m.id === choice.owner,
                          );
                          return (
                            <Tooltip
                              key={choice.owner ?? 'shared'}
                              text={choice.tooltip}
                            >
                              <button
                                type="button"
                                role="radio"
                                aria-checked={isChosen}
                                aria-label={choice.label}
                                // One tab stop per row, on its choice.
                                tabIndex={isChosen ? 0 : -1}
                                className={
                                  isChosen
                                    ? 'account-owners-choice account-owners-choice--chosen'
                                    : 'account-owners-choice'
                                }
                                onClick={() => choose(account, choice.owner)}
                              >
                                {member ? (
                                  <Avatar
                                    name={choice.label}
                                    src={pictureSrc(member.picture_url)}
                                    size="medium"
                                  />
                                ) : (
                                  <span className="account-owners-shared-icon">
                                    <FontAwesomeIcon icon={faLink} />
                                  </span>
                                )}
                                {isChosen && (
                                  <span
                                    className="account-owners-check"
                                    aria-hidden="true"
                                  >
                                    <FontAwesomeIcon icon={faCheck} />
                                  </span>
                                )}
                              </button>
                            </Tooltip>
                          );
                        })}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="modal-button modal-button--cancel"
            onClick={onClose}
          >
            {t('settings.institutions.owners.cancel')}
          </button>
          <button
            type="button"
            className="modal-button modal-button--primary"
            onClick={handleSave}
            disabled={updateOwnersMutation.isPending}
          >
            {t('settings.institutions.owners.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
