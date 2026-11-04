import { useState } from 'react';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import './Tooltip.css';

type TooltipProps = {
  /** Shown on hover or keyboard focus. A `\n` starts a new line. */
  text: string | undefined;
  children: ReactNode;
};

/**
 * Shows `text` above `children` as soon as they're hovered or focused, unlike a native `title`,
 * which waits about a second. Portaled to `document.body`, so a scrolling or clipped parent can't
 * cut it off. Only visual: give the control its own accessible name.
 */
export const Tooltip = ({ text, children }: TooltipProps) => {
  const [anchor, setAnchor] = useState<DOMRect | null>(null);

  const show = (element: HTMLElement) =>
    setAnchor(element.getBoundingClientRect());
  const hide = () => setAnchor(null);

  return (
    <span
      className="tooltip-anchor"
      onMouseEnter={(event) => show(event.currentTarget)}
      onMouseLeave={hide}
      onFocus={(event) => show(event.currentTarget)}
      onBlur={hide}
      // A click opens whatever the child is (a picker's list), which the tooltip would cover.
      onMouseDown={hide}
    >
      {children}
      {text &&
        anchor &&
        createPortal(
          <div
            className="tooltip"
            role="presentation"
            style={{
              left: anchor.left + anchor.width / 2,
              top: anchor.top,
            }}
          >
            {text}
          </div>,
          document.body,
        )}
    </span>
  );
};
