import { apiFetch } from '../../../lib/api-client';
import type { Category, CategoryGroup } from './types';

export interface NewCategoryGroup {
  name_en: string;
  name_fr: string;
  type: CategoryGroup['type'];
}

export const createCategoryGroup = (
  newGroup: NewCategoryGroup,
): Promise<CategoryGroup> =>
  apiFetch<CategoryGroup>('/category-groups', {
    method: 'POST',
    body: JSON.stringify(newGroup),
  });

/** Body for `PATCH /category-groups/{id}`. Unset fields are left unchanged. */
export interface CategoryGroupPatch {
  name_en?: string;
  name_fr?: string;
  type?: CategoryGroup['type'];
}

export const updateCategoryGroup = (
  id: number,
  patch: CategoryGroupPatch,
): Promise<CategoryGroup> =>
  apiFetch<CategoryGroup>(`/category-groups/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const deleteCategoryGroup = (id: number): Promise<void> =>
  apiFetch<void>(`/category-groups/${id}`, { method: 'DELETE' });

/** Sets the household's category group display order to exactly `categoryGroupIds` and returns
 * the groups in that new order. */
export const reorderCategoryGroups = (
  categoryGroupIds: number[],
): Promise<CategoryGroup[]> =>
  apiFetch<CategoryGroup[]>('/category-groups/reorder', {
    method: 'PATCH',
    body: JSON.stringify({ category_group_ids: categoryGroupIds }),
  });

export interface NewCategory {
  group_id: number;
  name_en: string;
  name_fr: string;
}

export const createCategory = (newCategory: NewCategory): Promise<Category> =>
  apiFetch<Category>('/categories', {
    method: 'POST',
    body: JSON.stringify(newCategory),
  });

/** Body for `PATCH /categories/{id}`. Unset fields are left unchanged. */
export interface CategoryPatch {
  group_id?: number;
  name_en?: string;
  name_fr?: string;
}

export const updateCategory = (
  id: number,
  patch: CategoryPatch,
): Promise<Category> =>
  apiFetch<Category>(`/categories/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

export const deleteCategory = (id: number): Promise<void> =>
  apiFetch<void>(`/categories/${id}`, { method: 'DELETE' });

/** Sets the display order of the given categories (a single group's ids, in their new order) and
 * returns every one of the household's categories in their (possibly unaffected) order. */
export const reorderCategories = (categoryIds: number[]): Promise<Category[]> =>
  apiFetch<Category[]>('/categories/reorder', {
    method: 'PATCH',
    body: JSON.stringify({ category_ids: categoryIds }),
  });
