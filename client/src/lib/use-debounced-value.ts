import { useEffect, useState } from 'react';

/** `value`, but only once it has stopped changing for `delayMs` — e.g. to ask the server about a
 * text field once the user pauses typing rather than on every keystroke. Compared by its JSON, so
 * a fresh object with the same content doesn't restart the wait. */
export const useDebouncedValue = <T>(value: T, delayMs: number): T => {
  const [debounced, setDebounced] = useState(value);
  const key = JSON.stringify(value);

  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, delayMs]);

  return debounced;
};
