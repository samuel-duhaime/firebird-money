import { toast } from 'sonner';
import i18n from '../i18n';

export const notImplementedToast = () =>
  toast.error(i18n.t('toast.notImplemented'));

export const signInFailedToast = () =>
  toast.error(i18n.t('toast.signInFailed'));

export const signOutFailedToast = () =>
  toast.error(i18n.t('toast.signOutFailed'));

export const onboardingFailedToast = () =>
  toast.error(i18n.t('toast.onboardingFailed'));

export const firstNameRequiredToast = () =>
  toast.error(i18n.t('toast.firstNameRequired'));

export const copyInviteCodeSucceededToast = () =>
  toast.success(i18n.t('toast.copyInviteCodeSucceeded'));

export const copyInviteCodeFailedToast = () =>
  toast.error(i18n.t('toast.copyInviteCodeFailed'));

export const joinCodeNotFoundToast = () =>
  toast.error(i18n.t('toast.joinCodeNotFound'));

export const downloadFailedToast = () =>
  toast.error(i18n.t('toast.downloadFailed'));

export const addTransactionSucceededToast = () =>
  toast.success(i18n.t('toast.addTransactionSucceeded'));

export const updateTransactionFailedToast = () =>
  toast.error(i18n.t('toast.updateTransactionFailed'));

export const deleteTransactionSucceededToast = () =>
  toast.success(i18n.t('toast.deleteTransactionSucceeded'));

export const deleteTransactionFailedToast = () =>
  toast.error(i18n.t('toast.deleteTransactionFailed'));

export const bulkUpdateTransactionsFailedToast = () =>
  toast.error(i18n.t('toast.bulkUpdateTransactionsFailed'));

export const noBulkChangesToast = () =>
  toast.error(i18n.t('transactions.editMultiple.noChangesError'));

export const bulkDeleteTransactionsSucceededToast = (count: number) =>
  toast.success(i18n.t('toast.bulkDeleteTransactionsSucceeded', { count }));

export const bulkDeleteTransactionsFailedToast = () =>
  toast.error(i18n.t('toast.bulkDeleteTransactionsFailed'));

export const invalidAmountToast = () =>
  toast.error(i18n.t('transactions.add.invalidAmount'));

export const amountTooLongToast = () =>
  toast.error(i18n.t('transactions.add.amountTooLong'));

export const requiredFieldToast = () =>
  toast.error(i18n.t('transactions.add.required'));

/** "13 transactions added · 2 new merchants · 1 new account" — each extra part only when some were
 * created. */
export const importSucceededToast = (
  createdCount: number,
  createdMerchantCount: number,
  createdAccountCount: number,
) => {
  const parts = [
    i18n.t('toast.importSucceeded', { count: createdCount }),
    createdMerchantCount > 0 &&
      i18n.t('toast.importNewMerchants', { count: createdMerchantCount }),
    createdAccountCount > 0 &&
      i18n.t('toast.importNewAccounts', { count: createdAccountCount }),
  ].filter(Boolean);
  toast.success(parts.join(' · '));
};

export const importFailedToast = () =>
  toast.error(i18n.t('toast.importFailed'));

export const deleteTagFailedToast = () =>
  toast.error(i18n.t('toast.deleteTagFailed'));

export const deleteMerchantFailedToast = () =>
  toast.error(i18n.t('toast.deleteMerchantFailed'));

export const deleteMerchantInUseToast = () =>
  toast.error(i18n.t('toast.deleteMerchantInUse'));

export const deleteAccountFailedToast = () =>
  toast.error(i18n.t('toast.deleteAccountFailed'));

export const deleteAccountInUseToast = () =>
  toast.error(i18n.t('toast.deleteAccountInUse'));

export const deleteInstitutionFailedToast = () =>
  toast.error(i18n.t('toast.deleteInstitutionFailed'));

export const updateSettingsFailedToast = () =>
  toast.error(i18n.t('toast.updateSettingsFailed'));

export const reorderTagsFailedToast = () =>
  toast.error(i18n.t('toast.reorderTagsFailed'));

export const deleteCategoryFailedToast = () =>
  toast.error(i18n.t('toast.deleteCategoryFailed'));

export const deleteCategoryGroupFailedToast = () =>
  toast.error(i18n.t('toast.deleteCategoryGroupFailed'));

export const reorderCategoriesFailedToast = () =>
  toast.error(i18n.t('toast.reorderCategoriesFailed'));

export const reorderCategoryGroupsFailedToast = () =>
  toast.error(i18n.t('toast.reorderCategoryGroupsFailed'));
