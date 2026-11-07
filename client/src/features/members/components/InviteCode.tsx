import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCopy, faRotate } from '@fortawesome/free-solid-svg-icons';
import { useCurrentUser } from '../../auth/hooks/use-current-user';
import { useRegenerateJoinCode } from '../hooks/use-household-membership';
import {
  copyInviteCodeFailedToast,
  copyInviteCodeSucceededToast,
} from '../../../lib/toast';
import './InviteCode.css';

/** The household's join code, which someone types during onboarding to join this household. */
export const InviteCode = () => {
  const { t } = useTranslation();
  const { data: session } = useCurrentUser();
  const regenerate = useRegenerateJoinCode();
  const household = session?.household;

  if (!household) return null;

  const joinCode = household.join_code;

  const copy = () =>
    navigator.clipboard
      .writeText(joinCode)
      .then(copyInviteCodeSucceededToast, copyInviteCodeFailedToast);

  return (
    <section className="invite-code" aria-labelledby="invite-code-heading">
      <div className="invite-code-text">
        <h3 id="invite-code-heading">{t('settings.members.invite.heading')}</h3>
        <p>{t('settings.members.invite.description')}</p>
      </div>
      <code className="invite-code-value">{joinCode}</code>
      <button type="button" className="invite-code-copy" onClick={copy}>
        <FontAwesomeIcon icon={faCopy} />
        {t('settings.members.invite.copy')}
      </button>
      {household.role === 'family_manager' && (
        <button
          type="button"
          className="invite-code-copy"
          onClick={() => regenerate.mutate(household.household_id)}
          disabled={regenerate.isPending}
          title={t('settings.members.invite.regenerateHint')}
        >
          <FontAwesomeIcon icon={faRotate} />
          {t('settings.members.invite.regenerate')}
        </button>
      )}
    </section>
  );
};
