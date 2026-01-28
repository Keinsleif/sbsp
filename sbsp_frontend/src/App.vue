<template>
  <component :is="connected ? MainView : ConnectView" />
  <update-dialog v-if="target == 'tauri'" v-model="uiState.isUpdateDialogOpen"></update-dialog>
  <credits-dialog v-if="target == 'tauri'" v-model="uiState.isCreditsDialogOpen"></credits-dialog>
  <license-dialog v-if="side == 'host'" v-model="uiState.isLicenseDialogOpen"></license-dialog>
</template>

<script setup lang="ts">
  import { onMounted, onUnmounted, ref, watch } from 'vue';
  import { useUiSettings } from './stores/uiSettings';
  import { useI18n } from 'vue-i18n';
  import { useTheme } from 'vuetify';
  import { useApi, side, target } from './api';
  import MainView from './MainView.vue';
  import ConnectView from './ConnectView.vue';
  import { createWindowMenu } from './window-menu';
  import UpdateDialog from './components/dialog/UpdateDialog.vue';
  import CreditsDialog from './components/dialog/CreditsDialog.vue';
  import LicenseDialog from './components/dialog/LicenseDialog.vue';
  import { useUiState } from './stores/uistate';

  const { locale } = useI18n({ useScope: 'global' });
  const windowMenu = createWindowMenu();
  const theme = useTheme();
  const uiState = useUiState();
  const uiSettings = useUiSettings();
  const api = useApi();
  const connected = ref(side != 'remote');
  let unlisten: (() => void) | null = null;

  watch(
    () => uiSettings.settings.appearance,
    (newSettings, oldSettings) => {
      if (newSettings.language != oldSettings.language) {
        setLanguage(newSettings.language);
      }
      if (newSettings.darkMode != oldSettings.darkMode) {
        theme.change(newSettings.darkMode);
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

  onMounted(() => {
    setLanguage(uiSettings.settings.appearance.language);
    theme.change(uiSettings.settings.appearance.darkMode);
    windowMenu?.init();
    if (side == 'remote') {
      api.remote
        ?.onConnectionStatusChanged((isConnected) => {
          connected.value = isConnected;
          windowMenu?.updateConnectionStatus(isConnected);
        })
        .then((ulfn) => (unlisten = ulfn));
      api.remote?.isConnected().then((isConnected) => {
        connected.value = isConnected;
        windowMenu?.updateConnectionStatus(isConnected);
      });
    }
  });
  onUnmounted(() => {
    if (unlisten != null) unlisten();
  });
</script>

<style>
  html {
    height: 100%;
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
  }
</style>
