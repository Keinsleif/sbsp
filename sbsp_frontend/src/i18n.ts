// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { createI18n } from 'vue-i18n';
import { en as vuetifyEn, ja as vuetifyJa } from 'vuetify/locale';

import en from './locales/en.json';
import ja from './locales/ja.json';

export const i18n = createI18n({
  legacy: false,
  locale: navigator.language,
  fallbackLocale: 'en',
  messages: {
    en: {
      $vuetify: {
        ...vuetifyEn,
      },
      ...en,
    },
    ja: {
      $vuetify: {
        ...vuetifyJa,
      },
      ...ja,
    },
  },
});
