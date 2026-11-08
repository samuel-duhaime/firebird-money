import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import {
  faAngleDown,
  faAngleUp,
  faGripVertical,
} from '@fortawesome/free-solid-svg-icons';
import { useDragReorder } from '../../../lib/use-drag-reorder';
import { reorderRulesFailedToast } from '../../../lib/toast';
import { useReorderRules } from '../hooks/use-reorder-rules';
import { useRuleLookups } from '../hooks/use-rule-lookups';
import { useRules } from '../hooks/use-rules';
import { ruleMatchesFilters } from '../utils/filter';
import type { RuleFilters } from '../utils/filter';
import type { Rule } from '../utils/types';
import { RuleSummary } from './RuleSummary';
import '../../../components/MoveButtons.css';
import './RulesList.css';

type RulesListProps = {
  filters: RuleFilters;
  search: string;
  onEdit: (rule: Rule) => void;
};

/** See TagsList: `useDragReorder` needs a stable array while `rules` loads. */
const NO_RULES: Rule[] = [];

/** "3 days ago", in the app's language. */
const timeAgo = (iso: string, locale: string) => {
  const seconds = (new Date(iso).getTime() - Date.now()) / 1000;
  const format = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  const steps: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31536000],
    ['month', 2592000],
    ['day', 86400],
    ['hour', 3600],
    ['minute', 60],
  ];
  for (const [unit, size] of steps) {
    if (Math.abs(seconds) >= size)
      return format.format(Math.round(seconds / size), unit);
  }
  return format.format(0, 'minute');
};

export const RulesList = ({ filters, search, onEdit }: RulesListProps) => {
  const { t, i18n } = useTranslation();
  const lookups = useRuleLookups();
  const { data: rules, isPending, isError } = useRules();
  const reorderRules = useReorderRules();
  const {
    items,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    moveUp,
    moveDown,
  } = useDragReorder(
    rules ?? NO_RULES,
    (rule) => rule.id,
    reorderRules,
    reorderRulesFailedToast,
  );

  if (isPending) return <p>{t('settings.rules.loading')}</p>;
  if (isError) return <p>{t('settings.rules.error')}</p>;
  if (rules.length === 0) return <p>{t('settings.rules.empty')}</p>;

  const visible = items
    .map((rule, index) => ({ rule, index }))
    .filter(({ rule }) => ruleMatchesFilters(rule, filters, search, lookups));
  // Moving a rule among a filtered few would quietly move it past the hidden ones too.
  const canReorder = visible.length === items.length;

  if (visible.length === 0) return <p>{t('settings.rules.noMatch')}</p>;

  return (
    <ul className="rules-list">
      {visible.map(({ rule, index }) => (
        <li
          key={rule.id}
          className="rules-list-item"
          onDragOver={canReorder ? handleDragOver(index) : undefined}
          onDrop={canReorder ? handleDrop : undefined}
        >
          <div className="rule-card">
            {canReorder && (
              <div className="rule-card-handle">
                <span
                  className="rules-list-drag-handle"
                  draggable
                  onDragStart={handleDragStart(index)}
                  onDragEnd={handleDragEnd}
                  aria-hidden="true"
                >
                  <FontAwesomeIcon icon={faGripVertical} />
                </span>
                <div className="move-buttons">
                  <button
                    type="button"
                    className="move-button"
                    onClick={() => moveUp(index)}
                    disabled={index === 0}
                    aria-label={t('settings.rules.moveUp', { n: index + 1 })}
                  >
                    <FontAwesomeIcon icon={faAngleUp} />
                  </button>
                  <button
                    type="button"
                    className="move-button"
                    onClick={() => moveDown(index)}
                    disabled={index === items.length - 1}
                    aria-label={t('settings.rules.moveDown', { n: index + 1 })}
                  >
                    <FontAwesomeIcon icon={faAngleDown} />
                  </button>
                </div>
              </div>
            )}
            <button
              type="button"
              className="rule-card-body"
              onClick={() => onEdit(rule)}
              aria-label={t('settings.rules.editRule', { n: index + 1 })}
            >
              <RuleSummary rule={rule} lookups={lookups} />
            </button>
          </div>
          {rule.last_applied_at && (
            <p className="rules-list-applied">
              {t('settings.rules.applied', {
                count: rule.last_applied_count,
                when: timeAgo(rule.last_applied_at, i18n.language),
              })}
            </p>
          )}
        </li>
      ))}
    </ul>
  );
};
