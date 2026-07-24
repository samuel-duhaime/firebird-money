import { toast } from 'sonner';

export const notImplementedToast = () =>
  toast.error('This feature is not available yet.');

export const downloadFailedToast = () =>
  toast.error('Failed to download transactions.');
