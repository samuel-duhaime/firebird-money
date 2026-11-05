import { useTranslation } from 'react-i18next';
import { faCheck, faUsers } from '@fortawesome/free-solid-svg-icons';
import { ChoiceIcon } from '../../members/components/OwnerPicker';
import type { PickerChoice } from '../../members/components/OwnerPicker';

/** Who a review waits on, besides a member: No one (it's reviewed) or Anyone. */
export type ReviewChoice = 'no_one' | 'anyone';

/** "No one" and "Anyone", for an `OwnerPicker` of who a review waits on. */
export const useReviewChoices = (): PickerChoice<ReviewChoice>[] => {
  const { t } = useTranslation();
  return [
    {
      value: 'no_one',
      label: t('transactions.review.noOne'),
      icon: <ChoiceIcon icon={faCheck} />,
    },
    {
      value: 'anyone',
      label: t('transactions.review.anyone'),
      icon: <ChoiceIcon icon={faUsers} />,
    },
  ];
};
