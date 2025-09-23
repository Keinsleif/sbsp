import { createI18n } from 'vue-i18n';
import { en, ja } from 'vuetify/locale';

export const i18n = createI18n({
  legacy: false,
  locale: navigator.language,
  fallbackLocale: 'en',
  messages: {
    en: {
      $vuetify: {
        ...en,
      },
      notification: {
        modelLoaded: 'Show model loaded.',
        modelSaved: 'Show model saved.',
      },
    },
    ja: {
      $vuetify: {
        ...ja,
      },
      notification: {
        modelLoaded: '正常にロードされました',
        modelSaved: '正常に保存されました',
      },
    },
  },
});
