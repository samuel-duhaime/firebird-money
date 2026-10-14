import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { InviteCode } from '../features/members/components/InviteCode';
import { MembersList } from '../features/members/components/MembersList';
import './_app.settings.css';

const MembersPage = () => {
  const { t } = useTranslation();

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.members.heading')}</h2>
      </div>

      <InviteCode />
      <MembersList />
    </div>
  );
};

export const Route = createFileRoute('/_app/settings/members')({
  component: MembersPage,
});
