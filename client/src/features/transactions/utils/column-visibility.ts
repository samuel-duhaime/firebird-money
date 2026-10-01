import type { Settings } from '../../settings/utils/types';
import type { SettingsPatch } from '../../settings/utils/api';

/** The transactions list's optional columns — every one can be hidden independently. */
export type OptionalColumn = 'tags' | 'category' | 'account';

export const OPTIONAL_COLUMNS: OptionalColumn[] = ['tags', 'category', 'account'];

export type ColumnVisibility = Record<OptionalColumn, boolean>;

const SETTINGS_FIELD: Record<OptionalColumn, keyof SettingsPatch> = {
  tags: 'show_tags_column',
  category: 'show_category_column',
  account: 'show_account_column',
};

/** Every column defaults to visible while settings haven't loaded yet, so the list doesn't flash
 * columns in once the real preference arrives. */
export const columnVisibilityFromSettings = (settings: Settings | undefined): ColumnVisibility => ({
  tags: settings?.show_tags_column ?? true,
  category: settings?.show_category_column ?? true,
  account: settings?.show_account_column ?? true,
});

/** The `PATCH /settings` body that flips a single column's visibility. */
export const toggleColumnPatch = (
  column: OptionalColumn,
  visibility: ColumnVisibility,
): SettingsPatch => ({
  [SETTINGS_FIELD[column]]: !visibility[column],
});

/** `grid-template-columns` for a transactions row, given which optional columns are currently
 * visible. Each visible optional column gets an equal-width track, so the number of tracks (not
 * their order) is all that depends on `visibility` — see `TransactionsList.css` for the fixed
 * tracks (merchant, amount, chevron). */
export const rowGridTemplateColumns = (
  visibility: ColumnVisibility,
  { selectable }: { selectable: boolean },
): string => {
  const optionalTracks = OPTIONAL_COLUMNS.filter((column) => visibility[column]).map(
    () => 'minmax(0, 1.3fr)',
  );
  const tracks = selectable
    ? ['20px', 'minmax(0, 2fr)', ...optionalTracks, '110px', '16px']
    : ['minmax(0, 2fr)', ...optionalTracks, '110px', '16px'];
  return tracks.join(' ');
};
