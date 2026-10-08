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

/** Creates a custom merchant for the signed-in household via `POST /merchants` and returns its
 * id. Needed wherever a test drives the UI's `MerchantPicker` directly (it only selects from
 * existing merchants — unlike the old free-text field, there's no "type a brand-new name and
 * submit" path in the picker itself; "Create new merchant" opens the settings page in a new tab
 * instead), so the name has to already exist before the picker is opened. */
export const createMerchant = async (
  request: APIRequestContext,
  apiOrigin: string,
  name: string,
): Promise<number> => {
  const response = await request.post(`${apiOrigin}/merchants`, {
    data: { name },
  });
  if (!response.ok()) {
    throw new Error(
      `failed to seed merchant "${name}": ${response.status()} ${await response.text()}`,
    );
  }
  const body: { id: number } = await response.json();
  return body.id;
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
      // Sent as original_statement, not merchant_id — the server's own matching/creation
      // resolves a merchant from it, same as a real import would. The real server binary runs
      // here (see fixtures/worker-infra.ts), so the common merchants ARE seeded — pick fixture
      // text that doesn't contain one of their names as a substring (e.g. not "Whole Foods
      // Market"/"Netflix Subscription"), or this will match the common merchant instead of
      // creating a new custom one, and `merchant_name` will differ from the text given here.
      original_statement: merchant,
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
