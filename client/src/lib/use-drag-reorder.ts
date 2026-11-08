import { useEffect, useRef, useState } from 'react';
import type { DragEvent } from 'react';

type ReorderMutationLike = {
  mutate: (
    ids: number[],
    options?: { onError?: () => void; onSettled?: () => void },
  ) => void;
};

/** Drives a reorder of `sourceItems` by drag-and-drop or, for anyone who can't or doesn't want to
 * drag (keyboard and screen-reader users in particular — a mouse-only drag handle leaves them with
 * no way to reorder at all), by the `moveUp`/`moveDown` pair. Generic over what's being reordered
 * (tags, category groups, or the categories inside one group).
 *
 * A local copy the gesture reorders live, independent of `sourceItems` until the change lands and
 * the server confirms it — kept in sync with `sourceItems` the rest of the time (e.g. after a
 * create/delete elsewhere), but never while a drag or a save is in flight, so neither a refetch
 * landing mid-drag nor one landing while the mutation is still pending clobbers it back to the old
 * order. */
export const useDragReorder = <T>(
  sourceItems: T[],
  getId: (item: T) => number,
  mutation: ReorderMutationLike,
  onError: () => void,
) => {
  const [items, setItems] = useState<T[]>(sourceItems);
  const isDraggingRef = useRef(false);
  const isSavingRef = useRef(false);
  const draggedIndexRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isDraggingRef.current && !isSavingRef.current) setItems(sourceItems);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceItems]);

  /** Saves `next` as the new order, unless it turns out to match `sourceItems` already (a drag
   * that landed back where it started, or a move past either end) — skipping the round trip (and
   * the error toast it'd show on a no-op failure) in that case. */
  const commit = (next: T[]) => {
    setItems(next);
    const unchanged =
      next.length === sourceItems.length &&
      next.every((item, index) => getId(item) === getId(sourceItems[index]));
    if (unchanged) return;

    isSavingRef.current = true;
    mutation.mutate(next.map(getId), {
      onSettled: () => {
        isSavingRef.current = false;
      },
      onError: () => {
        onError();
        setItems(sourceItems);
      },
    });
  };

  const handleDragStart =
    (index: number) => (event: DragEvent<HTMLSpanElement>) => {
      // Moving again before the last move is saved would compare against the old order.
      if (isSavingRef.current) {
        event.preventDefault();
        return;
      }
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

  const handleDragEnd = (event: DragEvent<HTMLElement>) => {
    isDraggingRef.current = false;
    draggedIndexRef.current = null;
    // `dragend` also fires for a canceled drag (Escape, or a release outside the list), with no
    // drop effect: put the order back instead of saving it. A drag that lands back on its starting
    // position is a no-op in `commit`.
    if (event.dataTransfer.dropEffect === 'none') setItems(sourceItems);
    else commit(items);
  };

  /** Moves the item at `index` one slot toward the start (`-1`) or end (`1`) and saves
   * immediately — a no-op past either end. */
  const move = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (isSavingRef.current || target < 0 || target >= items.length) return;
    const next = [...items];
    const [moved] = next.splice(index, 1);
    next.splice(target, 0, moved);
    commit(next);
  };

  return {
    items,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    moveUp: (index: number) => move(index, -1),
    moveDown: (index: number) => move(index, 1),
  };
};
