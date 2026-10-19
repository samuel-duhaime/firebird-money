import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { isSupportedLanguage } from '../../../i18n';
import { useSettings } from './use-settings';
import { useUpdateSettings } from './use-update-settings';

/**
 * Keeps the UI language in step with the signed-in user's settings, so it follows them across
 * devices. A saved language is applied as soon as settings load; a user who has never saved one
 * gets the language they're already reading (e.g. picked on the landing page) saved for them.
 *
 * Only reacts to the saved language changing, not to the UI language: the language switcher saves
 * its own choice, and reacting to the UI language here would switch it straight back before that
 * save lands.
 */
export const useSyncLanguage = () => {
  const { i18n } = useTranslation();
  const { data: settings } = useSettings();
  const { mutate: updateSettings } = useUpdateSettings();
  const savedLanguage = settings?.language;

  useEffect(() => {
    if (savedLanguage === undefined) {
      return;
    }

    if (savedLanguage === null) {
      const current = i18n.resolvedLanguage ?? i18n.language;
      if (isSupportedLanguage(current)) {
        updateSettings({ language: current });
      }
      return;
    }

    if (savedLanguage !== i18n.resolvedLanguage) {
      void i18n.changeLanguage(savedLanguage);
    }
    // Only the saved language should trigger this; see the doc comment above.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [savedLanguage]);
};
