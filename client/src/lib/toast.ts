import { toast } from 'sonner';

export const notImplementedToast = () =>
  toast.error('This feature is not available yet.');

export const downloadFailedToast = () =>
  toast.error('Failed to download transactions.');

export const importStartedToast = () =>
  toast('Import started — this can take a minute.');

export const importFailedToast = () =>
  toast.error('Failed to import transactions.');

export const importSucceededToast = (createdCount: number) =>
  toast.success(
    `Imported ${createdCount} transaction${createdCount === 1 ? '' : 's'}.`,
  );
