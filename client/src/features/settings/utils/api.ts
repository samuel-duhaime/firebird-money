import { apiFetch } from '../../../lib/api-client';
import type { SupportedLanguage } from '../../../i18n';
import type { Settings } from './types';

export const getSettings = (): Promise<Settings> => apiFetch<Settings>('/settings');

/** Body for `PATCH /settings`. Unset fields are left unchanged. */
export interface SettingsPatch {
  show_category_column?: boolean;
  show_tags_column?: boolean;
  show_account_column?: boolean;
  language?: SupportedLanguage;
}

export const updateSettings = (patch: SettingsPatch): Promise<Settings> =>
  apiFetch<Settings>('/settings', {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });
