// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { createApp } from 'vue';
import { createPinia } from 'pinia';
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate';
import App from './App.vue';
import { i18n } from './i18n';
import PrimeVue, { type PrimeVueConfiguration } from 'primevue/config';
import Aura from '@primeuix/themes/aura';

const app = createApp(App);

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);

app.use(pinia).use(i18n);

const primeVueConfig: PrimeVueConfiguration = {
  theme: {
    preset: Aura,
    options: {
      darkModeSelector: 'system',
      cssLayer: {
        name: 'primevue',
        order: 'base, components, primevue, utilities',
      },
    },
  },
  csp: {
    nonce: 'aYxPbxuXswGq0ST6rvQ2AA==',
  },
};

app.use(PrimeVue, primeVueConfig);

app.mount('#app');
