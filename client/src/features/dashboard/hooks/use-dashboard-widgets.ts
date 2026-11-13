import { useMemo } from 'react';
import { useSettings } from '../../settings/hooks/use-settings';
import { dashboardWidgetsFromSettings } from '../utils/widgets';

/** The user's dashboard widgets, in their order. The same array until settings change, as
 * `useDragReorder` needs. */
export const useDashboardWidgets = () => {
  const { data: settings } = useSettings();
  return useMemo(() => dashboardWidgetsFromSettings(settings), [settings]);
};
