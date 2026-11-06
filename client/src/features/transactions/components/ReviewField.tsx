import { useTranslation } from 'react-i18next';
import { OwnerPicker } from '../../members/components/OwnerPicker';
import { useReviewChoices } from '../hooks/use-review-choices';
import type { ReviewChoice } from '../hooks/use-review-choices';
import { useMembers } from '../../members/hooks/use-members';
import { memberDisplayName } from '../../members/utils/format';
import { useUpdateTransaction } from '../hooks/use-update-transaction';
import { updateTransactionFailedToast } from '../../../lib/toast';
import { toIntlLocale } from '../../../i18n/locale';
import type { Transaction } from '../utils/types';
import './ReviewField.css';

/** The edit panel's "Needs review by" field. Picking No one marks it reviewed by the caller; once
 * someone did, it says who and when underneath. */
export const ReviewField = ({ transaction }: { transaction: Transaction }) => {
  const { t, i18n } = useTranslation();
  const locale = toIntlLocale(i18n.resolvedLanguage ?? i18n.language);
  const { data: members } = useMembers();
  const choices = useReviewChoices();
  const updateTransactionMutation = useUpdateTransaction();

  const value: number | ReviewChoice = transaction.reviewed
    ? 'no_one'
    : (transaction.review_assignee_user_id ?? 'anyone');

  const handleChange = (next: number | ReviewChoice) =>
    updateTransactionMutation.mutate(
      {
        id: transaction.id,
        patch:
          next === 'no_one' ? { reviewed: true } : { review_assignee: next },
      },
      { onError: () => updateTransactionFailedToast() },
    );

  const reviewer = members?.find(
    (member) => member.id === transaction.reviewed_by_user_id,
  );

  return (
    <div className="edit-transaction-field">
      <label htmlFor="edit-review">
        {t('transactions.filters.needsReviewBy')}
      </label>
      <OwnerPicker
        id="edit-review"
        value={value}
        choices={choices}
        onChange={handleChange}
      />
      {transaction.reviewed && transaction.reviewed_at && (
        <p className="review-field-hint">
          {t('transactions.review.reviewedBy', {
            name: reviewer
              ? memberDisplayName(reviewer)
              : t('transactions.review.memberUnavailable'),
            date: new Date(transaction.reviewed_at).toLocaleDateString(locale, {
              month: 'short',
              day: 'numeric',
            }),
          })}
        </p>
      )}
    </div>
  );
};
