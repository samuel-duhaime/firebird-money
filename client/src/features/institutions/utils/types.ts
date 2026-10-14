export interface Institution {
  id: number;
  /** `null` for a common institution (Desjardins, RBC, ...), shared by every household and immune
   * to rename/delete through this API — everything else belongs to exactly one household. */
  household_id: number | null;
  name: string;
  /** Other names this institution is searchable by, e.g. "Banque Nationale" for "National Bank". */
  aliases: string[];
  website_url: string | null;
  logo_url: string | null;
  created_at: string;
  /** How many of this household's accounts are held here. */
  account_count: number;
}
