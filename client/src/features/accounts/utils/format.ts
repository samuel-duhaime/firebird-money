import type { Account } from './types';

/** "Compte conjoint (…5012)", or just the name when there's no `last_four`. */
export const accountLabel = (account: Pick<Account, 'name' | 'last_four'>) =>
  account.last_four ? `${account.name} (…${account.last_four})` : account.name;
