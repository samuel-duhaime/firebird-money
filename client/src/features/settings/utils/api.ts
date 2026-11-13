import { apiFetch } from '../../../lib/api-client';
import type { SupportedLanguage } from '../../../i18n';
import type { DashboardWidget, Settings } from './types';

export const getSettings = (): Promise<Settings> =>
  apiFetch<Settings>('/settings');

/** Body for `PATCH /settings`. Unset fields are left unchanged. */
export interface SettingsPatch {
  show_category_column?: boolean;
  show_tags_column?: boolean;
  show_account_column?: boolean;
  show_owner_column?: boolean;
  language?: SupportedLanguage;
  /** The whole list, in its new order. */
  dashboard_widgets?: DashboardWidget[];
}

export const updateSettings = (patch: SettingsPatch): Promise<Settings> =>
  apiFetch<Settings>('/settings', {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });
