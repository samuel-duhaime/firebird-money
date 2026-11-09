import { useTranslation } from 'react-i18next';
import { Avatar } from '../../../components/Avatar';
import {
  formatAmount,
  formatDateHeading,
} from '../../transactions/utils/format';
import { useRuleLookups } from '../hooks/use-rule-lookups';
import type { RulePreview } from '../utils/api';
import { TagDot } from './TagDot';
import './RulePreviewList.css';

/** Where the preview of the rule being edited stands: no condition yet, a switched-on condition
 * still incomplete, the request failed, the first one still loading, an earlier draft's results
 * while the latest edit loads (`stale`), or up to date. */
export type PreviewState =
  'none' | 'incomplete' | 'error' | 'loading' | 'stale' | 'ready';

type RulePreviewListProps = {
  state: PreviewState;
  preview: RulePreview | undefined;
  onRetry: () => void;
  locale: string;
};

/** The transactions a rule matches, grouped by date, each showing what the rule would make it —
 * read-only: what changes is highlighted, what it replaces is struck through. */
export const RulePreviewList = ({
  state,
  preview,
  onRetry,
  locale,
}: RulePreviewListProps) => {
  const { t } = useTranslation();
  const lookups = useRuleLookups();

  if (state === 'error') {
    return (
      <div className="rule-preview-status rule-preview-error">
        <span>{t('settings.rules.preview.failed')}</span>
        <button
          type="button"
          className="modal-button rule-preview-retry"
          onClick={onRetry}
        >
          {t('settings.rules.preview.retry')}
        </button>
      </div>
    );
  }
  if (state !== 'ready' && state !== 'stale') {
    return (
      <p className="rule-preview-status">
        {
          {
            none: t('settings.rules.preview.noCondition'),
            incomplete: t('settings.rules.preview.incomplete'),
            loading: t('settings.rules.preview.loading'),
          }[state]
        }
      </p>
    );
  }
  if (!preview) return null;
  if (preview.transactions.length === 0) {
    return (
      <p className="rule-preview-status">{t('settings.rules.preview.empty')}</p>
    );
  }

  const byDate: { date: string; rows: RulePreview['transactions'] }[] = [];
  for (const row of preview.transactions) {
    const last = byDate[byDate.length - 1];
    if (last?.date === row.before.date) last.rows.push(row);
    else byDate.push({ date: row.before.date, rows: [row] });
  }

  return (
    <div
      className={
        state === 'stale' ? 'rule-preview rule-preview--stale' : 'rule-preview'
      }
      aria-busy={state === 'stale'}
    >
      <p className="rule-preview-summary">
        {t('settings.rules.preview.summary', {
          count: preview.match_count,
          changed: preview.change_count,
        })}
        {preview.transactions.length < preview.match_count &&
          ` ${t('settings.rules.preview.limited', { count: preview.transactions.length })}`}
      </p>
      <ul className="rule-preview-list">
        {byDate.map((group) => (
          <li key={group.date}>
            <p className="rule-preview-date">
              {formatDateHeading(group.date, locale)}
            </p>
            <ul className="rule-preview-rows">
              {group.rows.map(({ before, after }) => {
                const merchantChanged =
                  after.merchant_id !== before.merchant_id;
                const categoryChanged =
                  after.category_id !== before.category_id;
                const ownerChanged =
                  after.owner_user_id !== before.owner_user_id;
                const reviewChanged =
                  after.reviewed !== before.reviewed ||
                  (!after.reviewed &&
                    after.review_assignee_user_id !==
                      before.review_assignee_user_id);
                const beforeTagIds = before.tags.map((tag) => tag.id);
                const addedTagIds = after.tag_ids.filter(
                  (id) => !beforeTagIds.includes(id),
                );
                const isCredit = before.category_type !== 'expense';
                return (
                  <li key={before.id} className="rule-preview-row">
                    <span className="rule-preview-merchant">
                      <Avatar
                        name={before.merchant_name}
                        src={before.merchant_logo_url}
                      />
                      <span className="rule-preview-cell">
                        {merchantChanged && (
                          <s className="rule-preview-old">
                            {before.merchant_name}
                          </s>
                        )}
                        <span
                          className={merchantChanged ? 'rule-preview-new' : ''}
                        >
                          {merchantChanged && after.merchant_id !== null
                            ? lookups.merchantName(after.merchant_id)
                            : before.merchant_name}
                        </span>
                      </span>
                    </span>
                    <span className="rule-preview-cell">
                      {categoryChanged && (
                        <s className="rule-preview-old">
                          {lookups.categoryName(before.category_id)}
                        </s>
                      )}
                      <span
                        className={categoryChanged ? 'rule-preview-new' : ''}
                      >
                        {lookups.categoryName(
                          after.category_id ?? before.category_id,
                        )}
                      </span>
                    </span>
                    <span className="rule-preview-extras">
                      {ownerChanged && (
                        <span className="rule-preview-pill rule-preview-new">
                          {lookups.ownerName(after.owner_user_id ?? 'shared')}
                        </span>
                      )}
                      {addedTagIds.map((id) => {
                        const tag = lookups.tag(id);
                        return (
                          <span
                            key={id}
                            className="rule-preview-pill rule-preview-new"
                          >
                            <TagDot color={tag?.color} />
                            {tag?.name ?? t('settings.rules.deleted')}
                          </span>
                        );
                      })}
                      {reviewChanged && (
                        <span className="rule-preview-pill rule-preview-new">
                          {after.reviewed
                            ? t('settings.rules.review.reviewed')
                            : after.review_assignee_user_id === null
                              ? t('settings.rules.review.needsReviewAnyone')
                              : t('settings.rules.review.needsReviewBy', {
                                  name: lookups.ownerName(
                                    after.review_assignee_user_id,
                                  ),
                                })}
                        </span>
                      )}
                    </span>
                    <span
                      className={
                        isCredit
                          ? 'rule-preview-amount rule-preview-amount--credit'
                          : 'rule-preview-amount'
                      }
                    >
                      {isCredit && '+'}
                      {formatAmount(Number(before.amount), locale)}
                    </span>
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ul>
    </div>
  );
};
