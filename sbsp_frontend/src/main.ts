import { createApp } from 'vue';
import App from './App.vue';

// Vuetify
import 'vuetify/styles';
import { createVuetify } from 'vuetify';
import { aliases, mdi } from 'vuetify/iconsets/mdi-svg';
import { createPinia } from 'pinia';
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate';
import router from './router';
import { i18n } from './i18n';
import { useI18n } from 'vue-i18n';
import { createVueI18nAdapter } from 'vuetify/locale/adapters/vue-i18n';

const vuetify = createVuetify({
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
  theme: {
    cspNonce: 'aYxPbxuXswGq0ST6rvQ2AA==',
    defaultTheme: 'system',
  },
  locale: {
    adapter: createVueI18nAdapter({ i18n, useI18n }),
  },
});

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);

createApp(App).use(i18n).use(vuetify).use(router).use(pinia).mount('#app');
