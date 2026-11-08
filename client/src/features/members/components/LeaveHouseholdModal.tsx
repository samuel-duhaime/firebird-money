import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DangerModal } from '../../../components/DangerModal';
import { useHouseholdTransactionCount } from '../hooks/use-household-transaction-count';
import { useLeaveHousehold } from '../hooks/use-household-membership';
import { DownloadAllTransactions } from './DownloadAllTransactions';

type LeaveHouseholdModalProps = {
  householdId: number;
  onClose: () => void;
};

/** Spells out what leaving does before it happens, and has the household's transactions
 * downloaded first: there's no undo, only the invite code. */
export const LeaveHouseholdModal = ({
  householdId,
  onClose,
}: LeaveHouseholdModalProps) => {
  const { t } = useTranslation();
  const leave = useLeaveHousehold();
  const count = useHouseholdTransactionCount();
  const [downloaded, setDownloaded] = useState(false);

  return (
    <DangerModal
      title={t('settings.members.household.leaveTitle')}
      acknowledgement={t('settings.members.household.leaveUnderstood')}
      confirmLabel={t('settings.members.household.leaveConfirm')}
      canConfirm={downloaded || count === 0}
      isPending={leave.isPending}
      onConfirm={() => leave.mutate(householdId)}
      onClose={onClose}
    >
      <p>{t('settings.members.household.leaveIntro')}</p>
      <ul>
        <li>{t('settings.members.household.leaveAccess')}</li>
        <li>{t('settings.members.household.leaveShared')}</li>
        <li>{t('settings.members.household.leaveReviews')}</li>
        <li>{t('settings.members.household.leaveComeBack')}</li>
      </ul>
      <DownloadAllTransactions
        count={count}
        onDownloaded={() => setDownloaded(true)}
      />
    </DangerModal>
  );
};
