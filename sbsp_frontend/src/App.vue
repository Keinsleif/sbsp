<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import MainView from './MainView.vue';
import ConnectView from './ConnectView.vue';
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { createWindowMenu } from './window-menu';
import { useUiSettings } from './stores/uiSettings';
import { useUiState } from './stores/uiState.ts';
import { useApi } from './api';
import { usePreferredColorScheme } from '@vueuse/core';
import { setTheme } from '@tauri-apps/api/app';
import Toast from 'primevue/toast';
import UpdateDialog from './components/dialog/UpdateDialog.vue';
import CreditsDialog from './components/dialog/CreditsDialog.vue';
import LicenseDialog from './components/dialog/LicenseDialog.vue';

const isTauri = __IS_TAURI__;
const isHost = __IS_HOST__;

const connected = ref<boolean>(__IS_HOST__);

const { locale } = useI18n({ useScope: 'global' });
const windowMenu = createWindowMenu();

const api = useApi();
const uiState = useUiState();
const uiSettings = useUiSettings();
const colorScheme = usePreferredColorScheme();

watch(
  [colorScheme, () => uiSettings.settings.appearance.darkMode],
  ([scheme, darkMode], oldValue) => {
    let isDark;
    if (__IS_HOST__ && darkMode !== oldValue[1]) {
      setTheme(darkMode === 'system' ? null : darkMode);
    }
    if (uiSettings.settings.appearance.darkMode === 'system') {
      isDark = scheme !== 'light';
    } else {
      isDark = darkMode !== 'light';
    }
    if (isDark) {
      document.documentElement.classList.add('app-dark');
    } else {
      document.documentElement.classList.remove('app-dark');
    }
  },
  {
    immediate: true,
  },
);

watch(
  () => uiSettings.settings.appearance,
  (newSettings, oldSettings) => {
    if (newSettings.language !== oldSettings.language) {
      setLanguage(newSettings.language);
    }
  },
);

watch(
  () => uiState.mode,
  (newMode) => {
    windowMenu?.updateEditMode(newMode);
  },
);

const setLanguage = (language: string | null) => {
  if (language != null) {
    locale.value = language;
  } else {
    locale.value = navigator.language;
  }
  windowMenu?.updateLocale();
};

let unlisten: (() => void) | null = null;

onMounted(() => {
  void Promise.resolve(windowMenu?.init()).then(() => {
    setLanguage(uiSettings.settings.appearance.language);
  });
  if (__IS_REMOTE__) {
    api.remote
      ?.onConnectionStatusChanged((isConnected, perm) => {
        connected.value = isConnected;
        uiState.setPermission(perm || 0);
        windowMenu?.updateConnectionStatus(isConnected);
      })
      .then((ulfn) => (unlisten = ulfn));
    api.remote?.isConnected().then(([isConnected, perm]) => {
      connected.value = isConnected;
      uiState.setPermission(perm || 0);
      windowMenu?.updateConnectionStatus(isConnected);
    });
  }
});
onUnmounted(() => {
  if (unlisten != null) unlisten();
});
</script>

<template>
  <component :is="connected ? MainView : ConnectView" />
  <Toast />
  <UpdateDialog
    v-if="isTauri"
    v-model="uiState.isUpdateDialogOpen"
  />
  <CreditsDialog
    v-if="isTauri"
    v-model="uiState.isCreditsDialogOpen"
  />
  <LicenseDialog
    v-if="isHost"
    v-model="uiState.isLicenseDialogOpen"
  />
</template>

<style>
html {
  height: 100vh;
  overflow: hidden;
  scrollbar-width: none;
  overscroll-behavior: none;
  user-select: none;
  -webkit-user-select: none;
  touch-action: none;
}

body,
#app {
  height: 100%;
  overflow: hidden;
}
</style>
