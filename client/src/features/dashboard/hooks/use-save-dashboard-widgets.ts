import { useUpdateSettings } from '../../settings/hooks/use-update-settings';
import { updateSettingsFailedToast } from '../../../lib/toast';
import type {
  DashboardWidget,
  DashboardWidgetId,
} from '../../settings/utils/types';

type SaveOptions = { onError?: () => void; onSettled?: () => void };

/** Saves the dashboard's widgets, with a toast when that fails. */
export const useSaveDashboardWidgets = () => {
  const updateSettingsMutation = useUpdateSettings();

  const save = (next: DashboardWidget[], options?: SaveOptions) =>
    updateSettingsMutation.mutate(
      { dashboard_widgets: next },
      {
        onError: () => {
          updateSettingsFailedToast();
          options?.onError?.();
        },
        onSettled: options?.onSettled,
      },
    );

  /** A `useDragReorder` mutation for `widgets`, or for just the ones in `ids` when they're a part
   * of it (the dashboard only lists the shown ones): those take their slots in their new order,
   * and every other widget keeps its own place, shown or hidden as it was. */
  const reorderMutation = (widgets: DashboardWidget[]) => ({
    mutate: (ids: DashboardWidgetId[], options?: SaveOptions) => {
      const moved = ids.map(
        (id) => widgets.find((widget) => widget.id === id) as DashboardWidget,
      );
      let next = 0;
      save(
        widgets.map((widget) =>
          ids.includes(widget.id) ? moved[next++] : widget,
        ),
        options,
      );
    },
  });

  return { save, reorderMutation };
};
