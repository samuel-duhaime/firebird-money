import { useTranslation } from 'react-i18next';
import { useHouseholdMembers } from '../hooks/use-household-members';
import type { HouseholdMember } from '../utils/types';
import './MembersList.css';

/** Name to show for a member: their full name, or their email if they never gave one (members who
 * joined before onboarding asked for a name, until they're next sent back to finish it). */
const displayName = (member: HouseholdMember) =>
  [member.first_name, member.last_name].filter(Boolean).join(' ') ||
  member.email;

export const MembersList = () => {
  const { t } = useTranslation();
  const { data: members, isPending, isError } = useHouseholdMembers();

  if (isPending) return <p>{t('settings.members.loading')}</p>;
  if (isError) return <p>{t('settings.members.error')}</p>;

  return (
    <ul className="members-list">
      {members.map((member) => {
        const name = displayName(member);

        return (
          <li key={member.id} className="members-list-row">
            <span className="members-list-avatar" aria-hidden="true">
              {name.charAt(0).toUpperCase()}
            </span>
            <div className="members-list-identity">
              <span className="members-list-name">
                {name}
                {member.type === 'family_manager' && (
                  <span className="members-list-role">
                    {t('settings.members.manager')}
                  </span>
                )}
              </span>
              <span className="members-list-email">{member.email}</span>
            </div>
            <span
              className={`members-list-status members-list-status--${member.status}`}
            >
              {t(`settings.members.status.${member.status}`)}
            </span>
          </li>
        );
      })}
    </ul>
  );
};
