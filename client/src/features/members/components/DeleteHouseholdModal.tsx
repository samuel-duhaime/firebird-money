import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DangerModal } from '../../../components/DangerModal';
import { useHouseholdTransactionCount } from '../hooks/use-household-transaction-count';
import { useDeleteHousehold } from '../hooks/use-household-membership';
import { DownloadAllTransactions } from './DownloadAllTransactions';

type DeleteHouseholdModalProps = {
  householdId: number;
  onClose: () => void;
};

/** Deleting a household its only member wants to leave, once its transactions are downloaded so
 * they can be imported again in another household. */
export const DeleteHouseholdModal = ({
  householdId,
  onClose,
}: DeleteHouseholdModalProps) => {
  const { t } = useTranslation();
  const deleteHousehold = useDeleteHousehold();
  const count = useHouseholdTransactionCount();
  const [downloaded, setDownloaded] = useState(false);

  return (
    <DangerModal
      title={t('settings.members.household.deleteTitle')}
      acknowledgement={t('settings.members.household.understood')}
      confirmLabel={t('settings.members.household.deleteConfirm')}
      canConfirm={downloaded || count === 0}
      isPending={deleteHousehold.isPending}
      onConfirm={() => deleteHousehold.mutate(householdId)}
      onClose={onClose}
    >
      <p>{t('settings.members.household.deleteIntro')}</p>
      <DownloadAllTransactions
        count={count}
        onDownloaded={() => setDownloaded(true)}
      />
    </DangerModal>
  );
};
