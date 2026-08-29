const API_BASE_URL = import.meta.env.VITE_API_BASE_URL as string;

/** Thrown by the fetch helpers below on a non-OK response; `status` is the real HTTP status, so
 * callers can branch on it instead of pattern-matching the message. */
export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

export const apiFetch = async <T>(
  path: string,
  init?: RequestInit,
): Promise<T> => {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    // Sends the session cookie: the API is a different origin, so the browser withholds it
    // otherwise.
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    ...init,
  });

  if (!response.ok) {
    throw new ApiError(
      response.status,
      `${init?.method ?? 'GET'} ${path} failed: ${response.status}`,
    );
  }

  // 204 has no body to parse (logout, deletes).
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
};

export const apiFetchUpload = async <T>(
  path: string,
  formData: FormData,
): Promise<T> => {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    // Sends the session cookie: the API is a different origin, so the browser withholds it
    // otherwise.
    credentials: 'include',
    method: 'POST',
    body: formData,
  });

  if (!response.ok) {
    throw new ApiError(response.status, `POST ${path} failed: ${response.status}`);
  }

  return response.json() as Promise<T>;
};

/** Extracts the `filename` param from a `Content-Disposition: attachment; filename="..."` header. */
const parseFilename = (contentDisposition: string | null): string | undefined =>
  contentDisposition?.match(/filename="([^"]+)"/)?.[1];

export const apiFetchFile = async (
  path: string,
): Promise<{ blob: Blob; filename?: string }> => {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    // Sends the session cookie: the API is a different origin, so the browser withholds it
    // otherwise.
    credentials: 'include',
  });

  if (!response.ok) {
    throw new ApiError(response.status, `GET ${path} failed: ${response.status}`);
  }

  return {
    blob: await response.blob(),
    filename: parseFilename(response.headers.get('Content-Disposition')),
  };
};
