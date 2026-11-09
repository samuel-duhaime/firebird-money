import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { previewRule } from '../utils/api';
import type { RuleActions, RuleConditions } from '../utils/types';

/** The preview of a rule being edited — off while it has no condition yet (`conditions` null). */
export const useRulePreview = (
  conditions: RuleConditions | null,
  actions: RuleActions,
) =>
  useQuery({
    queryKey: ['rules', 'preview', conditions, actions],
    queryFn: () => previewRule(conditions!, actions),
    enabled: conditions !== null,
    placeholderData: keepPreviousData,
  });
