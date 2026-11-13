import type { DragEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faGripVertical } from '@fortawesome/free-solid-svg-icons';
import './WidgetMoveHandle.css';

type WidgetMoveHandleProps = {
  /** The widget's name, e.g. "Transactions", for the grip's tooltip. */
  name: string;
  onDragStart: (event: DragEvent<HTMLSpanElement>) => void;
  onDragEnd: (event: DragEvent<HTMLElement>) => void;
};

/** The grip a dashboard widget is dragged by, in its header's left padding so the title never
 * moves. Mouse only: keyboard users reorder with the Customize window's up/down buttons. */
export const WidgetMoveHandle = ({
  name,
  onDragStart,
  onDragEnd,
}: WidgetMoveHandleProps) => {
  const { t } = useTranslation();

  return (
    <span
      className="widget-move-handle"
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      title={t('dashboard.move', { name })}
      aria-hidden="true"
    >
      <FontAwesomeIcon icon={faGripVertical} />
    </span>
  );
};
