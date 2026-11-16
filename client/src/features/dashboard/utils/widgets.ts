import type {
  DashboardWidget,
  DashboardWidgetId,
  Settings,
} from '../../settings/utils/types';

/** Every widget, in the order a new user sees them. */
export const DASHBOARD_WIDGET_IDS: DashboardWidgetId[] = ['transactions'];

/** The user's widgets, in their order. Every widget shows while settings haven't loaded yet, and
 * one the server doesn't know about yet is left out. */
export const dashboardWidgetsFromSettings = (
  settings: Settings | undefined,
): DashboardWidget[] =>
  settings?.dashboard_widgets.filter((widget) =>
    DASHBOARD_WIDGET_IDS.includes(widget.id),
  ) ?? DASHBOARD_WIDGET_IDS.map((id) => ({ id, visible: true }));
