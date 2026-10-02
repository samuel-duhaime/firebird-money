import { useEffect, useRef, useState } from 'react';
import type { DragEvent } from 'react';

type ReorderMutationLike = {
  mutate: (ids: number[], options?: { onError?: () => void }) => void;
};

/** Drives a drag-and-drop reorder of `sourceItems`, generic over what's being reordered (category
 * groups, or the categories inside one group) — the same gesture `TagsList` implements inline,
 * factored out since this page needs it once per section and once per group.
 *
 * A local copy the drag gesture reorders live, independent of `sourceItems` until the drop lands
 * and the server confirms it — kept in sync with `sourceItems` the rest of the time (e.g. after a
 * create/delete elsewhere), but never mid-drag, so an in-flight gesture isn't clobbered by a
 * refetch landing mid-drag. */
export const useDragReorder = <T>(
  sourceItems: T[],
  getId: (item: T) => number,
  mutation: ReorderMutationLike,
  onError: () => void,
) => {
  const [items, setItems] = useState<T[]>(sourceItems);
  const isDraggingRef = useRef(false);
  const draggedIndexRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isDraggingRef.current) setItems(sourceItems);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceItems]);

  const handleDragStart =
    (index: number) => (event: DragEvent<HTMLSpanElement>) => {
      draggedIndexRef.current = index;
      isDraggingRef.current = true;
      event.dataTransfer.effectAllowed = 'move';
      // Without this, the row's own `dragstart` also bubbles to an ancestor row (a category's
      // drag handle sits inside its group's card, which sits inside its section) and starts a
      // second, outer drag gesture for the same pointer.
      event.stopPropagation();
    };

  const handleDragOver =
    (index: number) => (event: DragEvent<HTMLLIElement>) => {
      event.preventDefault();
      event.stopPropagation();
      const draggedIndex = draggedIndexRef.current;
      if (draggedIndex === null || draggedIndex === index) return;
      setItems((previous) => {
        const next = [...previous];
        const [moved] = next.splice(draggedIndex, 1);
        next.splice(index, 0, moved);
        return next;
      });
      draggedIndexRef.current = index;
    };

  const handleDrop = (event: DragEvent<HTMLLIElement>) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const handleDragEnd = () => {
    isDraggingRef.current = false;
    draggedIndexRef.current = null;
    // `dragend` also fires for a canceled drag (e.g. Escape) and for a drag that lands back on
    // its starting position — neither actually changed the order, so skip the round trip (and the
    // error toast it'd show on a no-op failure) when `items` still matches the source.
    const unchanged =
      items.length === sourceItems.length &&
      items.every((item, index) => getId(item) === getId(sourceItems[index]));
    if (unchanged) return;

    mutation.mutate(items.map(getId), {
      onError: () => {
        onError();
        setItems(sourceItems);
      },
    });
  };

  return { items, handleDragStart, handleDragOver, handleDrop, handleDragEnd };
};
