import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { en } from './locales/en';
import { fr } from './locales/fr';

export const SUPPORTED_LANGUAGES = ['en', 'fr'] as const;
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];

// Only remembers the choice made on signed-out pages (the landing and sign-in pages); once signed
// in, the language saved in the user's settings wins — see `useSyncLanguage`.
const LANGUAGE_STORAGE_KEY = 'firebird-language';

export const isSupportedLanguage = (value: string | null): value is SupportedLanguage =>
  SUPPORTED_LANGUAGES.includes(value as SupportedLanguage);

const getInitialLanguage = (): SupportedLanguage => {
  const stored = localStorage.getItem(LANGUAGE_STORAGE_KEY);
  return isSupportedLanguage(stored) ? stored : 'en';
};

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    fr: { translation: fr },
  },
  lng: getInitialLanguage(),
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});

i18n.on('languageChanged', (language) => {
  localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
});

export default i18n;
