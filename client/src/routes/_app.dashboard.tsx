import { useMemo } from 'react';
import type { ComponentType, ReactNode } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { RecentTransactionsCard } from '../features/dashboard/components/RecentTransactionsCard';
import { CustomizeDashboardButton } from '../features/dashboard/components/CustomizeDashboardButton';
import { WidgetMoveHandle } from '../features/dashboard/components/WidgetMoveHandle';
import { useDashboardWidgets } from '../features/dashboard/hooks/use-dashboard-widgets';
import { useSaveDashboardWidgets } from '../features/dashboard/hooks/use-save-dashboard-widgets';
import { useDragReorder } from '../lib/use-drag-reorder';
import type {
  DashboardWidget,
  DashboardWidgetId,
} from '../features/settings/utils/types';
import '../components/TopMenu.css';
import './_app.dashboard.css';

const WIDGETS: Record<
  DashboardWidgetId,
  ComponentType<{ moveHandle: ReactNode }>
> = {
  transactions: RecentTransactionsCard,
};

const Dashboard = () => {
  const { t } = useTranslation();
  const widgets = useDashboardWidgets();
  const visible = useMemo(
    () => widgets.filter((widget) => widget.visible),
    [widgets],
  );
  const { reorderMutation } = useSaveDashboardWidgets();
  const { items, handleDragStart, handleDragOver, handleDrop, handleDragEnd } =
    useDragReorder<DashboardWidget, DashboardWidgetId>(
      visible,
      (widget) => widget.id,
      reorderMutation(widgets),
      // The save already shows the toast.
      () => {},
    );

  if (items.length === 0)
    return <p className="dashboard-empty">{t('dashboard.empty')}</p>;

  return (
    <ul className="dashboard-grid">
      {items.map(({ id }, index) => {
        const Widget = WIDGETS[id];
        return (
          <li
            key={id}
            className="dashboard-widget"
            onDragOver={handleDragOver(index)}
            onDrop={handleDrop}
          >
            <Widget
              moveHandle={
                <WidgetMoveHandle
                  name={t(`dashboard.widgets.${id}`)}
                  onDragStart={handleDragStart(index)}
                  onDragEnd={handleDragEnd}
                />
              }
            />
          </li>
        );
      })}
    </ul>
  );
};

export const Route = createFileRoute('/_app/dashboard')({
  component: Dashboard,
  staticData: {
    topMenuTitle: 'nav.dashboard',
    topMenuActions: CustomizeDashboardButton,
  },
});
