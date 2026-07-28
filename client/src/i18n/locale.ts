import type { SupportedLanguage } from './index';

/** Maps an app language to the Intl locale used for date/number formatting. */
const INTL_LOCALES: Record<SupportedLanguage, string> = {
  en: 'en-US',
  fr: 'fr-CA',
};

export const toIntlLocale = (language: string): string =>
  INTL_LOCALES[language as SupportedLanguage] ?? INTL_LOCALES.en;
