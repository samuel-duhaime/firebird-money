import { useCurrentUser } from '../../auth/hooks/use-current-user';
import { useTransactions } from './use-transactions';

/** The "All transactions" dropdown's counts: the whole household's unreviewed transactions, and
 * the ones waiting on the caller or on Anyone, whatever else narrows the list. */
export const useReviewCounts = () => {
  const { data: currentUser } = useCurrentUser();
  const {
    data: needsReview,
    isPending,
    isError,
  } = useTransactions({
    needs_review: 'all',
  });
  const counts = {
    needs_review: needsReview?.length ?? 0,
    mine:
      needsReview?.filter(
        (transaction) =>
          transaction.review_assignee_user_id === null ||
          transaction.review_assignee_user_id === currentUser?.user.id,
      ).length ?? 0,
  };
  return { counts, isPending, isError };
};
