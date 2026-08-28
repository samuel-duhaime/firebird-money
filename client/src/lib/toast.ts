import { toast } from 'sonner';
import i18n from '../i18n';
import { toIntlLocale } from '../i18n/locale';

export const notImplementedToast = () =>
  toast.error(i18n.t('toast.notImplemented'));

export const signInFailedToast = () =>
  toast.error(i18n.t('toast.signInFailed'));

export const onboardingFailedToast = () =>
  toast.error(i18n.t('toast.onboardingFailed'));

export const joinCodeNotFoundToast = () =>
  toast.error(i18n.t('toast.joinCodeNotFound'));

export const downloadFailedToast = () =>
  toast.error(i18n.t('toast.downloadFailed'));

export const addTransactionSucceededToast = () =>
  toast.success(i18n.t('toast.addTransactionSucceeded'));

export const importStartedToast = () => toast(i18n.t('toast.importStarted'));

export const importFailedToast = () =>
  toast.error(i18n.t('toast.importFailed'));

export const importSucceededToast = (createdCount: number) =>
  toast.success(i18n.t('toast.importSucceeded', { count: createdCount }));

export const importPartialToast = (
  createdCount: number,
  failedCount: number,
  skippedCount: number,
) => {
  const issues = [
    failedCount > 0
      ? i18n.t('toast.importFailedCount', { count: failedCount })
      : null,
    skippedCount > 0
      ? i18n.t('toast.importSkippedCount', { count: skippedCount })
      : null,
  ].filter((issue): issue is string => issue !== null);
  const locale = toIntlLocale(i18n.resolvedLanguage ?? i18n.language);
  toast.warning(
    i18n.t('toast.importPartial', {
      count: createdCount,
      issues: new Intl.ListFormat(locale, {
        style: 'long',
        type: 'conjunction',
      }).format(issues),
    }),
  );
};
