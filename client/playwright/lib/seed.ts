import type { APIRequestContext } from '@playwright/test';

/** Shared by the e2e suite and the README screenshot script — both need to look up a seeded
 * household's category id before posting a transaction against it. */
export const getCategoryId = async (
  request: APIRequestContext,
  apiOrigin: string,
  nameEn: string,
): Promise<number> => {
  const response = await request.get(`${apiOrigin}/categories`);
  const categories: { id: number; name_en: string }[] = await response.json();
  const match = categories.find((category) => category.name_en === nameEn);
  if (!match) throw new Error(`category not seeded: ${nameEn}`);
  return match.id;
};

/** Shared by the e2e suite — looks up a seeded household's starter tag id by name (see
 * `server/src/features/tags/defaults.rs`), mirroring `getCategoryId` above. */
export const getTagId = async (
  request: APIRequestContext,
  apiOrigin: string,
  name: string,
): Promise<number> => {
  const response = await request.get(`${apiOrigin}/tags`);
  const tags: { id: number; name: string }[] = await response.json();
  const match = tags.find((tag) => tag.name === name);
  if (!match) throw new Error(`tag not seeded: ${name}`);
  return match.id;
};

export type SeedTransaction = {
  date: string;
  merchant: string;
  amount: string;
  categoryId: number;
  account?: string;
  tagIds?: number[];
};

export const seedTransaction = async (
  request: APIRequestContext,
  apiOrigin: string,
  {
    date,
    merchant,
    amount,
    categoryId,
    account = 'Seed',
    tagIds,
  }: SeedTransaction,
): Promise<void> => {
  const response = await request.post(`${apiOrigin}/transactions`, {
    data: {
      date,
      merchant,
      amount,
      category_id: categoryId,
      account,
      tag_ids: tagIds,
    },
  });
  if (!response.ok()) {
    throw new Error(
      `failed to seed transaction "${merchant}": ${response.status()} ${await response.text()}`,
    );
  }
};
