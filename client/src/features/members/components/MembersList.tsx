import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Avatar } from '../../../components/Avatar';
import { useCurrentUser } from '../../auth/hooks/use-current-user';
import { useMembers } from '../hooks/use-members';
import { memberDisplayName, pictureSrc } from '../utils/format';
import { MemberEditModal } from './MemberEditModal';
import './MembersList.css';

export const MembersList = () => {
  const { t } = useTranslation();
  const { data: members, isPending, isError } = useMembers();
  const { data: session } = useCurrentUser();
  const [editingId, setEditingId] = useState<number | null>(null);

  if (isPending) return <p>{t('settings.members.loading')}</p>;
  if (isError) return <p>{t('settings.members.error')}</p>;

  const isManager = session?.household?.role === 'family_manager';
  // Looked up on every render, so the open modal shows a new picture as soon as it's saved.
  const editing = members.find((member) => member.id === editingId);

  return (
    <>
      <ul className="members-list">
        {members.map((member) => {
          const name = memberDisplayName(member);

          return (
            <li key={member.id} className="members-list-row">
              <Avatar
                name={name}
                src={pictureSrc(member.picture_url)}
                size="medium"
              />
              <div className="members-list-identity">
                <span className="members-list-name">
                  {name}
                  <span className="members-list-role">
                    {member.role === 'family_manager'
                      ? t('settings.members.manager')
                      : t('settings.members.member')}
                  </span>
                </span>
                <span className="members-list-email">{member.email}</span>
              </div>
              <span
                className={`members-list-status members-list-status--${member.status}`}
              >
                {t(`settings.members.status.${member.status}`)}
              </span>
              {(isManager || member.id === session?.user.id) && (
                <button
                  type="button"
                  className="members-list-edit"
                  onClick={() => setEditingId(member.id)}
                >
                  {t('settings.members.edit')}
                </button>
              )}
            </li>
          );
        })}
      </ul>
      {editing && (
        <MemberEditModal member={editing} onClose={() => setEditingId(null)} />
      )}
    </>
  );
};
