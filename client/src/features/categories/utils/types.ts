/** A group of categories, e.g. "Food & Dining" — every category inside it shares its `type`. */
export interface CategoryGroup {
  id: number;
  name_en: string;
  name_fr: string;
  type: 'income' | 'expense' | 'transfer';
  created_at: string;
  /** This household's chosen display order for its groups (see `reorderCategoryGroups`) — lower
   * sorts first. Not a global rank, only meaningful relative to the household's other groups. */
  sort_order: number;
}

export interface Category {
  id: number;
  group_id: number;
  name_en: string;
  name_fr: string;
  created_at: string;
  /** This household's chosen display order for this category among the others in its group (see
   * `reorderCategories`) — lower sorts first. Only meaningful relative to the other categories
   * sharing its `group_id`. */
  sort_order: number;
}
