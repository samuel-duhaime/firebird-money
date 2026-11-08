import './TagDot.css';

/** A tag's color, as a small dot before its name. */
export const TagDot = ({ color }: { color: string | undefined }) => (
  <span
    className="rule-tag-dot"
    style={{ backgroundColor: color }}
    aria-hidden="true"
  />
);
