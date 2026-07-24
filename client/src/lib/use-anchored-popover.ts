import { useEffect, useRef, useState } from 'react';

type Position = { top: number; left: number };

/**
 * State + positioning for a button-triggered popover that's portaled to `document.body` (so it
 * can't be clipped by a scrolling ancestor). Tracks the trigger's position on scroll/resize, and
 * closes on outside click or Escape.
 */
export const useAnchoredPopover = <
  TriggerElement extends HTMLElement = HTMLDivElement,
  PopoverElement extends HTMLElement = HTMLDivElement,
>() => {
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState<Position | null>(null);
  const triggerRef = useRef<TriggerElement>(null);
  const popoverRef = useRef<PopoverElement>(null);

  useEffect(() => {
    if (!isOpen) return;

    const updatePosition = () => {
      const rect = triggerRef.current?.getBoundingClientRect();
      if (rect) setPosition({ top: rect.bottom + 8, left: rect.left });
    };
    updatePosition();

    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (triggerRef.current?.contains(target) || popoverRef.current?.contains(target)) return;
      setIsOpen(false);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsOpen(false);
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  return { isOpen, setIsOpen, position, triggerRef, popoverRef };
};
