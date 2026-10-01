import { useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { TagModal } from '../features/tags/components/TagModal';
import { TagsList } from '../features/tags/components/TagsList';
import type { Tag } from '../features/tags/utils/types';
import './_app.settings.css';

const TagsPage = () => {
  const { t } = useTranslation();
  const [modalTag, setModalTag] = useState<Tag | 'new' | null>(null);

  return (
    <div className="settings-panel">
      <div className="settings-panel-header">
        <h2>{t('settings.tags.heading')}</h2>
        <button
          type="button"
          className="settings-panel-primary-button"
          onClick={() => setModalTag('new')}
        >
          {t('settings.tags.newTag')}
        </button>
      </div>

      <TagsList onEdit={setModalTag} />

      {modalTag && (
        <TagModal
          tag={modalTag === 'new' ? undefined : modalTag}
          onClose={() => setModalTag(null)}
        />
      )}
    </div>
  );
};

export const Route = createFileRoute('/_app/settings/tags')({
  component: TagsPage,
});
