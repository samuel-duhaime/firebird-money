import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { SearchButton } from '../components/SearchButton';
import { RuleModal } from '../features/rules/components/RuleModal';
import { RulesFiltersButton } from '../features/rules/components/RulesFiltersButton';
import { RulesList } from '../features/rules/components/RulesList';
import { useRules } from '../features/rules/hooks/use-rules';
import {
  clearSuggestedRule,
  onSuggestedRule,
  suggestedRule,
} from '../features/rules/utils/suggested-rule';
import type { RuleDraft } from '../features/rules/utils/draft';
import { NO_RULE_FILTERS } from '../features/rules/utils/filter';
import type { RuleFilters } from '../features/rules/utils/filter';
import type { Rule } from '../features/rules/utils/types';
import './_app.settings.css';
import './_app.settings.rules.css';

type ModalState = { rule?: Rule; draft?: RuleDraft } | null;

const RulesPage = () => {
  const { t } = useTranslation();
  const { data: rules } = useRules();
  // A rule suggested from a transaction edit opens right away, already filled in.
  const [modal, setModal] = useState<ModalState>(() => {
    const draft = suggestedRule();
    return draft ? { draft } : null;
  });
  useEffect(clearSuggestedRule, []);
  useEffect(
    () =>
      onSuggestedRule(() => {
        const draft = suggestedRule();
        if (!draft) return;
        setModal({ draft });
        clearSuggestedRule();
      }),
    [],
  );
  const [search, setSearch] = useState('');
  const [filters, setFilters] = useState<RuleFilters>(NO_RULE_FILTERS);

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.rules.heading')}</h2>
        <button
          type="button"
          className="settings-panel-primary-button"
          onClick={() => setModal({})}
        >
          {t('settings.rules.newRule')}
        </button>
      </div>
      <p className="settings-panel-description">
        {t('settings.rules.description')}
      </p>

      <div className="rules-toolbar">
        <span className="rules-toolbar-count">
          {t('settings.rules.count', { count: rules?.length ?? 0 })}
        </span>
        <div className="rules-toolbar-actions">
          <SearchButton
            value={search || undefined}
            help={t('settings.rules.searchHelp')}
            onChange={(term) => setSearch(term ?? '')}
          />
          <RulesFiltersButton filters={filters} onChange={setFilters} />
        </div>
      </div>

      <RulesList
        filters={filters}
        search={search}
        onEdit={(rule) => setModal({ rule })}
      />

      {modal && (
        <RuleModal
          rule={modal.rule}
          initialDraft={modal.draft}
          onClose={() => setModal(null)}
        />
      )}
    </div>
  );
};

export const Route = createFileRoute('/_app/settings/rules')({
  component: RulesPage,
});
