/** Every value `accounts.type` accepts, in the order the type picker lists them. */
export const ACCOUNT_TYPES = [
  'checking',
  'savings',
  'credit_card',
  'investment',
  'loan',
  'mortgage',
  'cash',
  'other',
] as const;

export type AccountType = (typeof ACCOUNT_TYPES)[number];

export interface Account {
  id: number;
  household_id: number;
  institution_id: number;
  institution_name: string;
  /** What the household calls it, e.g. "Compte conjoint". */
  name: string;
  type: AccountType;
  /** The last few characters of the account number, shown as "(…5012)". */
  last_four: string | null;
  /** ISO 4217 code, e.g. "CAD". */
  currency: string;
  created_at: string;
  /** How many of this household's transactions are in this account. */
  transaction_count: number;
}
