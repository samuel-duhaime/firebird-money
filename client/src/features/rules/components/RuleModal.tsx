import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCircleMinus, faCirclePlus } from '@fortawesome/free-solid-svg-icons';
import { MultiSelect } from '../../../components/MultiSelect';
import { Select } from '../../../components/Select';
import { Switch } from '../../../components/Switch';
import { useDebouncedValue } from '../../../lib/use-debounced-value';
import { OwnerPicker } from '../../members/components/OwnerPicker';
import { toOwner } from '../../accounts/utils/api';
import { CategoryPicker } from '../../transactions/components/CategoryPicker';
import { MerchantPicker } from '../../transactions/components/MerchantPicker';
import { TagPicker } from '../../tags/components/TagPicker';
import { useDeleteRule } from '../hooks/use-delete-rule';
import { useRuleLookups } from '../hooks/use-rule-lookups';
import { useRulePreview } from '../hooks/use-rule-preview';
import { useSaveRule } from '../hooks/use-save-rule';
import {
  draftActions,
  draftConditions,
  emptyDraft,
  hasAction,
  hasIncompleteCondition,
  ruleToDraft,
} from '../utils/draft';
import type { ReviewChoice, RuleDraft } from '../utils/draft';
import type { AmountOperator, Rule, TextOperator } from '../utils/types';
import { RulePreviewList } from './RulePreviewList';
import type { PreviewState } from './RulePreviewList';
import '../../transactions/components/AddTransactionModal.css';
import '../../transactions/components/EditTransactionModal.css';
import './RuleModal.css';

const FOCUSABLE_SELECTOR =
  'input, select, button, [href], [tabindex]:not([tabindex="-1"])';

/** Popovers portal to `document.body`, outside the dialog, but still belong to its focus trap. */
const POPOVER_SELECTOR =
  '.category-picker-popover, .merchant-picker-popover, .tag-picker-popover, .owner-picker-popover, .multi-select-popover';

const AMOUNT_OPERATORS: AmountOperator[] = [
  'greater_than',
  'less_than',
  'equal_to',
  'between',
];

type RuleModalProps = {
  /** The rule to edit, or what to start a new one from (a `Partial` draft, e.g. from a
   * transaction edit) — a new empty rule when absent. */
  rule?: Rule;
  initialDraft?: RuleDraft;
  onClose: () => void;
};

/** One condition or action: a title with its switch, and its fields once it's on. */
const Section = ({
  title,
  on,
  onToggle,
  children,
}: {
  title: string;
  on: boolean;
  onToggle: (on: boolean) => void;
  children?: ReactNode;
}) => (
  <div className={on ? 'rule-section rule-section--on' : 'rule-section'}>
    <div className="rule-section-header">
      <span className="rule-section-title">{title}</span>
      <Switch checked={on} onChange={onToggle} ariaLabel={title} />
    </div>
    {on && children && <div className="rule-section-body">{children}</div>}
  </div>
);

export const RuleModal = ({ rule, initialDraft, onClose }: RuleModalProps) => {
  const { t, i18n } = useTranslation();
  const lookups = useRuleLookups();
  const [draft, setDraft] = useState<RuleDraft>(() =>
    rule ? ruleToDraft(rule) : (initialDraft ?? emptyDraft()),
  );
  const [tab, setTab] = useState<'settings' | 'preview'>('settings');
  // Off by default when editing, so tweaking a rule never rewrites past transactions by surprise.
  const [applyToExisting, setApplyToExisting] = useState(!rule);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  const saveRule = useSaveRule();
  const deleteRule = useDeleteRule();

  const update = (patch: Partial<RuleDraft>) => {
    setError('');
    setDraft((previous) => ({ ...previous, ...patch }));
  };

  const conditions = draftConditions(draft);
  const actions = draftActions(draft);
  // Typing in a text or amount field shouldn't ask the server on every keystroke.
  const debounced = useDebouncedValue({ conditions, actions }, 300);
  const {
    data: preview,
    isFetching,
    isError,
    refetch,
  } = useRulePreview(debounced.conditions, debounced.actions);
  // Whether the preview shown is the one for the draft as it is now: not while the debounce or the
  // request for the latest edit is still pending.
  const isCurrent =
    !isFetching &&
    JSON.stringify(debounced) === JSON.stringify({ conditions, actions });
  const previewState: PreviewState = !conditions
    ? 'none'
    : hasIncompleteCondition(draft)
      ? 'incomplete'
      : isError && !isFetching
        ? 'error'
        : !preview
          ? 'loading'
          : isCurrent
            ? 'ready'
            : 'stale';
  const matchCount = preview?.match_count ?? 0;

  useEffect(() => {
    dialogRef.current?.querySelector<HTMLElement>(FOCUSABLE_SELECTOR)?.focus();
  }, []);

  const handleKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key !== 'Tab') return;

    const focusable = [
      ...Array.from(
        dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR) ??
          [],
      ),
      ...Array.from(document.querySelectorAll(POPOVER_SELECTOR)).flatMap(
        (popover) =>
          Array.from(popover.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)),
      ),
      // A disabled control (e.g. Save while saving) can't take focus, so it can't end the trap.
    ].filter((element) => !element.matches(':disabled'));
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  const handleSave = () => {
    if (!conditions) {
      setError(t('settings.rules.modal.needCondition'));
      return;
    }
    if (hasIncompleteCondition(draft)) {
      setError(t('settings.rules.modal.incompleteCondition'));
      return;
    }
    const incompleteAction =
      (draft.renameOn && draft.merchantId === null) ||
      (draft.categoryOn && draft.categoryId === null) ||
      (draft.ownerOn && draft.owner === null) ||
      (draft.tagsOn && draft.tagIds.length === 0);
    if (incompleteAction) {
      setError(t('settings.rules.modal.incompleteAction'));
      return;
    }
    if (!hasAction(actions)) {
      setError(t('settings.rules.modal.needAction'));
      return;
    }
    saveRule.mutate(
      {
        id: rule?.id,
        input: { conditions, actions, apply_to_existing: applyToExisting },
      },
      {
        onSuccess: onClose,
        onError: () => setError(t('settings.rules.modal.failed')),
      },
    );
  };

  const handleDelete = () => {
    if (!rule) return;
    deleteRule.mutate(rule.id, {
      onSuccess: onClose,
      onError: () => {
        setConfirmingDelete(false);
        setError(t('settings.rules.modal.deleteFailed'));
      },
    });
  };

  const setStatement = (
    index: number,
    patch: { operator?: TextOperator; value?: string },
  ) =>
    update({
      statement: draft.statement.map((text, i) =>
        i === index ? { ...text, ...patch } : text,
      ),
    });

  const categoryLabel =
    draft.categoryId === null
      ? t('settings.rules.modal.changeCategoryTo')
      : lookups.categoryName(draft.categoryId);
  const merchantLabel =
    draft.merchantId === null
      ? t('settings.rules.modal.renameTo')
      : lookups.merchantName(draft.merchantId);

  const reviewValue = (choice: ReviewChoice) => String(choice);

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="rule-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="rule-modal-title"
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="rule-modal-title">
            {rule
              ? t('settings.rules.modal.editTitle')
              : t('settings.rules.modal.newTitle')}
          </h2>
          <button
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label={t('transactions.add.close')}
          >
            ×
          </button>
        </div>

        <div className="rule-modal-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'settings'}
            className={
              tab === 'settings'
                ? 'rule-modal-tab rule-modal-tab--active'
                : 'rule-modal-tab'
            }
            onClick={() => setTab('settings')}
          >
            {t('settings.rules.modal.settingsTab')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'preview'}
            className={
              tab === 'preview'
                ? 'rule-modal-tab rule-modal-tab--active'
                : 'rule-modal-tab'
            }
            onClick={() => setTab('preview')}
          >
            {t('settings.rules.modal.previewTab')}
            {previewState === 'ready' && (
              <span className="rule-modal-tab-count">{matchCount}</span>
            )}
          </button>
        </div>

        {tab === 'settings' ? (
          <div className="rule-modal-columns">
            <div className="rule-modal-column">
              <h3 className="rule-modal-column-title">
                {t('settings.rules.modal.ifTitle')}
              </h3>

              <Section
                title={t('settings.rules.fields.originalStatement')}
                on={draft.statementOn}
                onToggle={(on) => update({ statementOn: on })}
              >
                {draft.statement.map((text, index) => (
                  <div key={index}>
                    {index > 0 && (
                      <p className="rule-or">{t('settings.rules.or')}</p>
                    )}
                    <div className="rule-text-row">
                      <Select
                        value={text.operator}
                        aria-label={t('settings.rules.modal.operator')}
                        onChange={(e) =>
                          setStatement(index, {
                            operator: e.target.value as TextOperator,
                          })
                        }
                      >
                        <option value="contains">
                          {t('settings.rules.operators.contains')}
                        </option>
                        <option value="equals">
                          {t('settings.rules.operators.equals')}
                        </option>
                      </Select>
                      <input
                        type="text"
                        className="rule-input"
                        value={text.value}
                        placeholder={t(
                          'settings.rules.fields.originalStatement',
                        )}
                        aria-label={t(
                          'settings.rules.fields.originalStatement',
                        )}
                        onChange={(e) =>
                          setStatement(index, { value: e.target.value })
                        }
                      />
                      <div className="rule-text-row-buttons">
                        {draft.statement.length > 1 && (
                          <button
                            type="button"
                            className="rule-icon-button"
                            aria-label={t('settings.rules.modal.removeText')}
                            onClick={() =>
                              update({
                                statement: draft.statement.filter(
                                  (_, i) => i !== index,
                                ),
                              })
                            }
                          >
                            <FontAwesomeIcon icon={faCircleMinus} />
                          </button>
                        )}
                        {index === draft.statement.length - 1 && (
                          <button
                            type="button"
                            className="rule-icon-button"
                            aria-label={t('settings.rules.modal.addText')}
                            onClick={() =>
                              update({
                                statement: [
                                  ...draft.statement,
                                  { operator: 'contains', value: '' },
                                ],
                              })
                            }
                          >
                            <FontAwesomeIcon icon={faCirclePlus} />
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </Section>

              <Section
                title={t('settings.rules.fields.merchants')}
                on={draft.merchantsOn}
                onToggle={(on) => update({ merchantsOn: on })}
              >
                <MultiSelect
                  options={lookups.merchantOptions}
                  selected={draft.merchantIds}
                  onChange={(merchantIds) => update({ merchantIds })}
                  placeholder={t('settings.rules.modal.merchantIs')}
                  ariaLabel={t('settings.rules.fields.merchants')}
                  triggerClassName="modal-category-trigger"
                />
              </Section>

              <Section
                title={t('settings.rules.fields.amount')}
                on={draft.amountOn}
                onToggle={(on) => update({ amountOn: on })}
              >
                <div className="rule-amount-row">
                  <Select
                    value={draft.amountOperator}
                    aria-label={t('settings.rules.modal.operator')}
                    onChange={(e) =>
                      update({
                        amountOperator: e.target.value as AmountOperator,
                      })
                    }
                  >
                    {AMOUNT_OPERATORS.map((operator) => (
                      <option key={operator} value={operator}>
                        {t(`settings.rules.operators.${operator}`)}
                      </option>
                    ))}
                  </Select>
                  <div className="amount-input-wrapper">
                    <span className="amount-prefix" aria-hidden="true">
                      $
                    </span>
                    <input
                      type="text"
                      inputMode="decimal"
                      className="rule-input rule-input--amount"
                      placeholder="0.00"
                      value={draft.amount}
                      aria-label={t('settings.rules.fields.amount')}
                      onChange={(e) => update({ amount: e.target.value })}
                    />
                  </div>
                  {draft.amountOperator === 'between' && (
                    <>
                      <span className="rule-and">
                        {t('settings.rules.and')}
                      </span>
                      <div className="amount-input-wrapper">
                        <span className="amount-prefix" aria-hidden="true">
                          $
                        </span>
                        <input
                          type="text"
                          inputMode="decimal"
                          className="rule-input rule-input--amount"
                          placeholder="0.00"
                          value={draft.amountMax}
                          aria-label={t('settings.rules.modal.amountMax')}
                          onChange={(e) =>
                            update({ amountMax: e.target.value })
                          }
                        />
                      </div>
                    </>
                  )}
                </div>
              </Section>

              <Section
                title={t('settings.rules.fields.categories')}
                on={draft.categoriesOn}
                onToggle={(on) => update({ categoriesOn: on })}
              >
                <MultiSelect
                  options={lookups.categoryOptions}
                  selected={draft.categoryIds}
                  onChange={(categoryIds) => update({ categoryIds })}
                  placeholder={t('settings.rules.modal.categoryIs')}
                  ariaLabel={t('settings.rules.fields.categories')}
                  triggerClassName="modal-category-trigger"
                />
              </Section>

              <Section
                title={t('settings.rules.fields.owners')}
                on={draft.ownersOn}
                onToggle={(on) => update({ ownersOn: on })}
              >
                <MultiSelect
                  options={lookups.ownerOptions}
                  selected={draft.owners}
                  onChange={(owners) => update({ owners })}
                  placeholder={t('settings.rules.modal.ownerIs')}
                  ariaLabel={t('settings.rules.fields.owners')}
                  triggerClassName="modal-category-trigger"
                />
              </Section>

              <Section
                title={t('settings.rules.fields.accounts')}
                on={draft.accountsOn}
                onToggle={(on) => update({ accountsOn: on })}
              >
                <MultiSelect
                  options={lookups.accountOptions}
                  selected={draft.accountIds}
                  onChange={(accountIds) => update({ accountIds })}
                  placeholder={t('settings.rules.modal.accountIs')}
                  ariaLabel={t('settings.rules.fields.accounts')}
                  triggerClassName="modal-category-trigger"
                />
              </Section>
            </div>

            <div className="rule-modal-column">
              <h3 className="rule-modal-column-title">
                {t('settings.rules.modal.thenTitle')}
              </h3>

              <Section
                title={t('settings.rules.actions.rename')}
                on={draft.renameOn}
                onToggle={(on) => update({ renameOn: on })}
              >
                <MerchantPicker
                  merchantId={draft.merchantId}
                  label={merchantLabel}
                  className={
                    draft.merchantId === null
                      ? 'modal-category-trigger modal-category-trigger--placeholder'
                      : 'modal-category-trigger'
                  }
                  ariaLabel={t('settings.rules.actions.rename')}
                  onSelect={(merchantId) => update({ merchantId })}
                />
              </Section>

              <Section
                title={t('settings.rules.actions.category')}
                on={draft.categoryOn}
                onToggle={(on) => update({ categoryOn: on })}
              >
                <CategoryPicker
                  categoryId={draft.categoryId}
                  label={categoryLabel}
                  className={
                    draft.categoryId === null
                      ? 'modal-category-trigger modal-category-trigger--placeholder'
                      : 'modal-category-trigger'
                  }
                  ariaLabel={t('settings.rules.actions.category')}
                  onSelect={(categoryId) => update({ categoryId })}
                />
              </Section>

              <Section
                title={t('settings.rules.actions.owner')}
                on={draft.ownerOn}
                onToggle={(on) => update({ ownerOn: on })}
              >
                <OwnerPicker
                  value={
                    draft.owner === null
                      ? undefined
                      : draft.owner === 'shared'
                        ? null
                        : draft.owner
                  }
                  placeholder={t('settings.rules.modal.changeOwnerTo')}
                  ariaLabel={t('settings.rules.actions.owner')}
                  onChange={(owner) => update({ owner: toOwner(owner) })}
                />
              </Section>

              <Section
                title={t('settings.rules.actions.tags')}
                on={draft.tagsOn}
                onToggle={(on) => update({ tagsOn: on })}
              >
                <TagPicker
                  selectedTagIds={draft.tagIds}
                  triggerClassName="modal-category-trigger"
                  ariaLabel={t('settings.rules.actions.tags')}
                  onToggle={(tagId) =>
                    update({
                      tagIds: draft.tagIds.includes(tagId)
                        ? draft.tagIds.filter((id) => id !== tagId)
                        : [...draft.tagIds, tagId],
                    })
                  }
                />
              </Section>

              <Section
                title={t('settings.rules.actions.review')}
                on={draft.reviewOn}
                onToggle={(on) => update({ reviewOn: on })}
              >
                <Select
                  value={reviewValue(draft.review)}
                  aria-label={t('settings.rules.actions.review')}
                  onChange={(e) => {
                    const value = e.target.value;
                    update({
                      review:
                        value === 'reviewed' || value === 'anyone'
                          ? value
                          : Number(value),
                    });
                  }}
                >
                  <option value="reviewed">
                    {t('settings.rules.review.reviewed')}
                  </option>
                  <option value="anyone">
                    {t('settings.rules.review.needsReviewAnyone')}
                  </option>
                  {lookups.members.map((member) => (
                    <option key={member.id} value={member.id}>
                      {t('settings.rules.review.needsReviewBy', {
                        name: lookups.ownerName(member.id),
                      })}
                    </option>
                  ))}
                </Select>
              </Section>
            </div>
          </div>
        ) : (
          <RulePreviewList
            state={previewState}
            preview={preview}
            onRetry={() => refetch()}
            locale={i18n.language}
          />
        )}

        <div className="rule-modal-apply">
          <label className="rule-modal-apply-label">
            <input
              type="checkbox"
              checked={applyToExisting}
              onChange={(e) => setApplyToExisting(e.target.checked)}
            />
            {t('settings.rules.modal.applyToExisting')}
          </label>
          <span className="rule-modal-match-count">
            {
              {
                none: t('settings.rules.modal.noMatchYet'),
                incomplete: t('settings.rules.preview.incomplete'),
                error: t('settings.rules.preview.failed'),
                loading: t('settings.rules.preview.loading'),
                stale: t('settings.rules.preview.loading'),
                ready: t('settings.rules.modal.matchCount', {
                  count: matchCount,
                }),
              }[previewState]
            }
          </span>
        </div>

        <div className="modal-footer rule-modal-footer">
          {error && (
            <p className="modal-error rule-modal-error" role="alert">
              {error}
            </p>
          )}
          {rule &&
            (confirmingDelete ? (
              <div className="rule-modal-delete-confirm">
                <span>{t('settings.rules.deleteConfirm')}</span>
                <button
                  type="button"
                  className="modal-button modal-button--cancel"
                  onClick={() => setConfirmingDelete(false)}
                >
                  {t('settings.rules.modal.cancel')}
                </button>
                <button
                  type="button"
                  className="modal-button modal-button--delete-confirm"
                  onClick={handleDelete}
                  disabled={deleteRule.isPending}
                >
                  {t('settings.rules.delete')}
                </button>
              </div>
            ) : (
              <button
                type="button"
                className="modal-button modal-button--delete"
                onClick={() => setConfirmingDelete(true)}
              >
                {t('settings.rules.delete')}
              </button>
            ))}
          <button
            type="button"
            className="modal-button modal-button--primary rule-modal-save"
            onClick={handleSave}
            // Applying to existing transactions waits for the preview of the draft as it is now.
            disabled={
              saveRule.isPending ||
              (applyToExisting &&
                (previewState === 'loading' || previewState === 'stale'))
            }
          >
            {t('settings.rules.modal.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
