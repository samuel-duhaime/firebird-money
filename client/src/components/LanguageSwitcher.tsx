import { useTranslation } from 'react-i18next';
import { SUPPORTED_LANGUAGES, type SupportedLanguage } from '../i18n';
import './LanguageSwitcher.css';

interface LanguageSwitcherProps {
  /** Called after the UI switches language, e.g. to save the choice to the user's settings. */
  onChange?: (language: SupportedLanguage) => void;
}

/** Minimalist en/fr selector; will grow into a full language picker later. */
export const LanguageSwitcher = ({ onChange }: LanguageSwitcherProps) => {
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
          onClick={() => {
            void i18n.changeLanguage(language);
            onChange?.(language);
          }}
        >
          {language}
        </button>
      ))}
    </div>
  );
};
