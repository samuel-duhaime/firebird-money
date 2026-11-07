import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useCurrentUser } from '../../auth/hooks/use-current-user';
import { useMembers } from '../hooks/use-members';
import { DeleteHouseholdModal } from './DeleteHouseholdModal';
import { LeaveHouseholdModal } from './LeaveHouseholdModal';
import '../../transactions/components/AddTransactionModal.css';
import '../../transactions/components/EditTransactionModal.css';
import './HouseholdSection.css';

/**
 * Leaving the household, at the bottom of the Members page. What it offers depends on who's
 * asking: anyone can leave while a manager stays behind, the last manager has to name another one
 * first, and someone alone in the household deletes it instead. Both go through a `DangerModal`.
 */
export const HouseholdSection = () => {
  const { t } = useTranslation();
  const { data: session } = useCurrentUser();
  const { data: members } = useMembers();
  const [leaving, setLeaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const household = session?.household;
  if (!household || !members) return null;

  const isAlone = members.length === 1;
  const isLastManager =
    household.role === 'family_manager' &&
    members.filter((member) => member.role === 'family_manager').length === 1;

  return (
    <section
      className="household-section"
      aria-labelledby="household-section-heading"
    >
      <div className="household-section-text">
        <h3 id="household-section-heading">
          {t('settings.members.household.heading')}
        </h3>
        <p>
          {isAlone
            ? t('settings.members.household.aloneDescription')
            : isLastManager
              ? t('settings.members.household.lastManagerDescription')
              : t('settings.members.household.leaveDescription')}
        </p>
      </div>

      {isAlone ? (
        <button
          type="button"
          className="modal-button modal-button--delete"
          onClick={() => setDeleting(true)}
        >
          {t('settings.members.household.delete')}
        </button>
      ) : (
        <button
          type="button"
          className="modal-button modal-button--delete"
          onClick={() => setLeaving(true)}
          disabled={isLastManager}
        >
          {t('settings.members.household.leave')}
        </button>
      )}

      {leaving && (
        <LeaveHouseholdModal
          householdId={household.household_id}
          onClose={() => setLeaving(false)}
        />
      )}
      {deleting && (
        <DeleteHouseholdModal
          householdId={household.household_id}
          onClose={() => setDeleting(false)}
        />
      )}
    </section>
  );
};
