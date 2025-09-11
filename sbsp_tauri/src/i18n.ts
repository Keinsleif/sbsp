import { createI18n } from 'vue-i18n';

export const i18n = createI18n({
  legacy: false,
  locale: navigator.language,
  fallbackLocale: 'en',
  messages: {
    en: {
      notification: {
        modelLoaded: 'Show model loaded.',
        modelSaved: 'Show model saved.',
      },
    },
  },
});
