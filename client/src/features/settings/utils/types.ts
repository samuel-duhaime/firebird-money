import type { SupportedLanguage } from '../../../i18n';

/** A widget the dashboard can show. */
export type DashboardWidgetId = 'transactions';

/** One dashboard widget, shown or hidden; its place in the list is its place on the dashboard. */
export interface DashboardWidget {
  id: DashboardWidgetId;
  visible: boolean;
}

export interface Settings {
  user_id: number;
  show_category_column: boolean;
  show_tags_column: boolean;
  show_account_column: boolean;
  show_owner_column: boolean;
  /** `null` until the user picks a language. */
  language: SupportedLanguage | null;
  /** Every dashboard widget, in display order. */
  dashboard_widgets: DashboardWidget[];
}
