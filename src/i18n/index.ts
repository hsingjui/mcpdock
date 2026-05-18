import { createI18n } from 'vue-i18n';
import { localeMessages, type SupportedLocale } from '../locales';

const savedLocale = localStorage.getItem('mcpdock:locale');

function resolveSystemLocale(): SupportedLocale {
  const lang = navigator.language || 'en';
  if (lang.startsWith('zh')) return 'zh-CN';
  return 'en';
}

const initialLocale: SupportedLocale =
  savedLocale === 'zh-CN' || savedLocale === 'en' ? savedLocale : resolveSystemLocale();

const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: 'en',
  messages: localeMessages,
});

export { i18n, resolveSystemLocale };
