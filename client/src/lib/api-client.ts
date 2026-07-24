const API_BASE_URL = import.meta.env.VITE_API_BASE_URL as string;

export const apiFetch = async <T>(
  path: string,
  init?: RequestInit,
): Promise<T> => {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  });

  if (!response.ok) {
    throw new Error(
      `${init?.method ?? 'GET'} ${path} failed: ${response.status}`,
    );
  }

  return response.json() as Promise<T>;
};

/** Extracts the `filename` param from a `Content-Disposition: attachment; filename="..."` header. */
const parseFilename = (contentDisposition: string | null): string | undefined =>
  contentDisposition?.match(/filename="([^"]+)"/)?.[1];

export const apiFetchFile = async (
  path: string,
): Promise<{ blob: Blob; filename?: string }> => {
  const response = await fetch(`${API_BASE_URL}${path}`);

  if (!response.ok) {
    throw new Error(`GET ${path} failed: ${response.status}`);
  }

  return {
    blob: await response.blob(),
    filename: parseFilename(response.headers.get('Content-Disposition')),
  };
};
