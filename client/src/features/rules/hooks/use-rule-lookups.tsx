import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Avatar } from '../../../components/Avatar';
import type { MultiSelectOption } from '../../../components/MultiSelect';
import { useAccounts } from '../../accounts/hooks/use-accounts';
import type { Owner } from '../../accounts/utils/api';
import { accountLabel } from '../../accounts/utils/format';
import { useCategories } from '../../categories/hooks/use-categories';
import { useCategoryGroups } from '../../categories/hooks/use-category-groups';
import { SharedIcon } from '../../members/components/OwnerPicker';
import { useMembers } from '../../members/hooks/use-members';
import { memberDisplayName, pictureSrc } from '../../members/utils/format';
import { useMerchants } from '../../merchants/hooks/use-merchants';
import { useTags } from '../../tags/hooks/use-tags';
import { TagDot } from '../components/TagDot';

/** The household's merchants, categories, accounts, members, and tags, as names to show in a rule
 * and as options to pick in one. An id that no longer exists (deleted since the rule was saved)
 * shows as "Deleted". */
export const useRuleLookups = () => {
  const { t, i18n } = useTranslation();
  const { data: merchants } = useMerchants();
  const { data: categories } = useCategories();
  const { data: groups } = useCategoryGroups();
  const { data: accounts } = useAccounts();
  const { data: members } = useMembers();
  const { data: tags } = useTags();
  const isFrench = i18n.language === 'fr';

  return useMemo(() => {
    const deleted = t('settings.rules.deleted');
    const groupOrder = new Map(
      (groups ?? []).map((group) => [group.id, group]),
    );
    // Groups come in the API's order. Their `sort_order` alone can tie across types, which would
    // split a group's categories apart.
    const groupIndex = new Map(
      (groups ?? []).map((group, index) => [group.id, index]),
    );
    const sortedCategories = [...(categories ?? [])].sort(
      (a, b) =>
        (groupIndex.get(a.group_id) ?? 0) - (groupIndex.get(b.group_id) ?? 0) ||
        a.sort_order - b.sort_order,
    );

    const merchantName = (id: number) =>
      merchants?.find((merchant) => merchant.id === id)?.name ?? deleted;
    const categoryName = (id: number) => {
      const category = categories?.find((c) => c.id === id);
      if (!category) return deleted;
      return isFrench ? category.name_fr : category.name_en;
    };
    const accountName = (id: number) => {
      const account = accounts?.find((a) => a.id === id);
      return account ? accountLabel(account) : deleted;
    };
    const ownerName = (owner: Owner) => {
      if (owner === 'shared') return t('settings.rules.shared');
      const member = members?.find((m) => m.id === owner);
      return member ? memberDisplayName(member) : deleted;
    };
    const tag = (id: number) => tags?.find((tg) => tg.id === id);

    const merchantOptions: MultiSelectOption<number>[] = (merchants ?? []).map(
      (merchant) => ({
        value: merchant.id,
        label: merchant.name,
        icon: <Avatar name={merchant.name} src={merchant.logo_url} />,
      }),
    );
    const categoryOptions: MultiSelectOption<number>[] = sortedCategories.map(
      (category) => {
        const group = groupOrder.get(category.group_id);
        return {
          value: category.id,
          label: isFrench ? category.name_fr : category.name_en,
          group: group ? (isFrench ? group.name_fr : group.name_en) : undefined,
        };
      },
    );
    const accountOptions: MultiSelectOption<number>[] = (accounts ?? []).map(
      (account) => ({
        value: account.id,
        label: `${account.institution_name} — ${accountLabel(account)}`,
      }),
    );
    const ownerOptions: MultiSelectOption<Owner>[] = [
      {
        value: 'shared',
        label: t('settings.rules.shared'),
        icon: <SharedIcon />,
      },
      ...(members ?? []).map((member) => ({
        value: member.id,
        label: memberDisplayName(member),
        icon: (
          <Avatar
            name={memberDisplayName(member)}
            src={pictureSrc(member.picture_url)}
          />
        ),
      })),
    ];
    const tagOptions: MultiSelectOption<number>[] = (tags ?? []).map((tg) => ({
      value: tg.id,
      label: tg.name,
      icon: <TagDot color={tg.color} />,
    }));

    return {
      merchantName,
      categoryName,
      accountName,
      ownerName,
      tag,
      merchantOptions,
      categoryOptions,
      accountOptions,
      ownerOptions,
      tagOptions,
      members: members ?? [],
    };
  }, [t, isFrench, merchants, categories, groups, accounts, members, tags]);
};

export type RuleLookups = ReturnType<typeof useRuleLookups>;
