const API_BASE_URL = import.meta.env.SERVER_API_BASE_URL as string;

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
