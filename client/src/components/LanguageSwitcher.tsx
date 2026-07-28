import { useTranslation } from 'react-i18next';
import { SUPPORTED_LANGUAGES } from '../i18n';
import './LanguageSwitcher.css';

/** Minimalist en/fr selector; will grow into a full language picker later. */
export const LanguageSwitcher = () => {
  const { t, i18n } = useTranslation();

  return (
    <div className="language-switcher" role="group" aria-label={t('leftMenu.language')}>
      {SUPPORTED_LANGUAGES.map((language) => (
        <button
          key={language}
          type="button"
          className={
            i18n.resolvedLanguage === language
              ? 'language-switcher-option language-switcher-option--active'
              : 'language-switcher-option'
          }
          aria-pressed={i18n.resolvedLanguage === language}
          onClick={() => i18n.changeLanguage(language)}
        >
          {language}
        </button>
      ))}
    </div>
  );
};
