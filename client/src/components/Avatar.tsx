import { useState } from 'react';
import './Avatar.css';

type AvatarProps = {
  /** Its first letter stands in when there's no picture. */
  name: string;
  src?: string | null;
  size?: 'small' | 'medium' | 'large';
};

/**
 * A round picture of a person, merchant, or account, falling back to the first letter of its name
 * when there's no picture or it fails to load. Decorative: show the name next to it.
 */
export const Avatar = ({ name, src, size = 'small' }: AvatarProps) => {
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const className = `avatar avatar--${size}`;

  return src && src !== failedSrc ? (
    <img
      className={className}
      src={src}
      alt=""
      onError={() => setFailedSrc(src)}
    />
  ) : (
    <span className={className} aria-hidden="true">
      {name.charAt(0).toUpperCase()}
    </span>
  );
};
