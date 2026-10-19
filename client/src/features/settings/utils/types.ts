import type { SupportedLanguage } from '../../../i18n';

export interface Settings {
  user_id: number;
  show_category_column: boolean;
  show_tags_column: boolean;
  show_account_column: boolean;
  /** `null` until the user picks a language. */
  language: SupportedLanguage | null;
}
