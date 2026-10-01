export interface Tag {
  id: number;
  name: string;
  color: string;
  created_at: string;
  /** How many transactions currently carry this tag. */
  transaction_count: number;
  /** This household's chosen display order — lower sorts first. Not a global rank, only
   * meaningful relative to the household's other tags. */
  sort_order: number;
}
