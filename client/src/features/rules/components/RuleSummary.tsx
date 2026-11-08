import { Fragment } from 'react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { formatAmount } from '../../transactions/utils/format';
import { TagDot } from './TagDot';
import type { RuleLookups } from '../hooks/use-rule-lookups';
import type { Rule } from '../utils/types';

const Chip = ({ children }: { children: ReactNode }) => (
  <span className="rule-chip">{children}</span>
);

/** Several values joined by a muted "or". */
const AnyOf = ({ values, or }: { values: ReactNode[]; or: string }) => (
  <>
    {values.map((value, index) => (
      <Fragment key={index}>
        {index > 0 && <span className="rule-word">{or}</span>}
        {value}
      </Fragment>
    ))}
  </>
);

/** A rule in words, as chips: one "If …" line per condition, one line per action. */
export const RuleSummary = ({
  rule,
  lookups,
}: {
  rule: Rule;
  lookups: RuleLookups;
}) => {
  const { t, i18n } = useTranslation();
  const { conditions, actions } = rule;
  const or = t('settings.rules.or');
  const money = (value: string) => formatAmount(Number(value), i18n.language);

  const ifLines: { field: string; body: ReactNode }[] = [];
  if (conditions.original_statement) {
    ifLines.push({
      field: t('settings.rules.fields.originalStatement'),
      body: (
        <AnyOf
          or={or}
          values={conditions.original_statement.map((text, index) => (
            <Fragment key={index}>
              <Chip>{t(`settings.rules.operators.${text.operator}`)}</Chip>
              <Chip>{text.value}</Chip>
            </Fragment>
          ))}
        />
      ),
    });
  }
  const idLine = (field: string, names: string[]) =>
    ifLines.push({
      field,
      body: (
        <>
          <Chip>{t('settings.rules.is')}</Chip>
          <AnyOf
            or={or}
            values={names.map((name) => (
              <Chip key={name}>{name}</Chip>
            ))}
          />
        </>
      ),
    });
  if (conditions.merchant_ids) {
    idLine(
      t('settings.rules.fields.merchants'),
      conditions.merchant_ids.map(lookups.merchantName),
    );
  }
  if (conditions.amount) {
    const { operator, value, value_max } = conditions.amount;
    ifLines.push({
      field: t('settings.rules.fields.amount'),
      body: (
        <>
          <Chip>{t(`settings.rules.operators.${operator}`)}</Chip>
          <Chip>{money(value)}</Chip>
          {operator === 'between' && value_max !== null && (
            <>
              <span className="rule-word">{t('settings.rules.and')}</span>
              <Chip>{money(value_max)}</Chip>
            </>
          )}
        </>
      ),
    });
  }
  if (conditions.category_ids) {
    idLine(
      t('settings.rules.fields.categories'),
      conditions.category_ids.map(lookups.categoryName),
    );
  }
  if (conditions.owners) {
    idLine(
      t('settings.rules.fields.owners'),
      conditions.owners.map(lookups.ownerName),
    );
  }
  if (conditions.account_ids) {
    idLine(
      t('settings.rules.fields.accounts'),
      conditions.account_ids.map(lookups.accountName),
    );
  }

  const thenLines: { label: string; body: ReactNode }[] = [];
  if (actions.merchant_id !== null) {
    thenLines.push({
      label: t('settings.rules.summary.renameTo'),
      body: <Chip>{lookups.merchantName(actions.merchant_id)}</Chip>,
    });
  }
  if (actions.category_id !== null) {
    thenLines.push({
      label: t('settings.rules.summary.recategorizeTo'),
      body: <Chip>{lookups.categoryName(actions.category_id)}</Chip>,
    });
  }
  if (actions.owner !== null) {
    thenLines.push({
      label: t('settings.rules.summary.ownerTo'),
      body: <Chip>{lookups.ownerName(actions.owner)}</Chip>,
    });
  }
  if (actions.tag_ids) {
    thenLines.push({
      label: t('settings.rules.summary.addTags', {
        count: actions.tag_ids.length,
      }),
      body: actions.tag_ids.map((id) => {
        const tag = lookups.tag(id);
        return (
          <Chip key={id}>
            <TagDot color={tag?.color} />
            {tag?.name ?? t('settings.rules.deleted')}
          </Chip>
        );
      }),
    });
  }
  if (actions.review) {
    const { reviewed, assignee } = actions.review;
    thenLines.push({
      label: t('settings.rules.summary.reviewTo'),
      body: (
        <Chip>
          {reviewed
            ? t('settings.rules.review.reviewed')
            : assignee === null || assignee === 'anyone'
              ? t('settings.rules.review.needsReviewAnyone')
              : t('settings.rules.review.needsReviewBy', {
                  name: lookups.ownerName(assignee),
                })}
        </Chip>
      ),
    });
  }

  return (
    <>
      <div className="rule-card-side">
        {ifLines.map((line) => (
          <p key={line.field} className="rule-line">
            <span className="rule-word">{t('settings.rules.if')}</span>
            <Chip>{line.field}</Chip>
            {line.body}
          </p>
        ))}
      </div>
      <div className="rule-card-arrow" aria-hidden="true">
        →
      </div>
      <div className="rule-card-side">
        {thenLines.map((line) => (
          <p key={line.label} className="rule-line">
            <span className="rule-word rule-word--strong">{line.label}</span>
            {line.body}
          </p>
        ))}
      </div>
    </>
  );
};
