<template>
  <component :is="connected ? MainView : ConnectView" />
</template>

<script setup lang="ts">
  import { onMounted, onUnmounted, ref, watch } from 'vue';
  import { useUiSettings } from './stores/uiSettings';
  import { useI18n } from 'vue-i18n';
  import { useTheme } from 'vuetify';
  import { useApi, side } from './api';
  import MainView from './MainView.vue';
  import ConnectView from './ConnectView.vue';

  const { locale } = useI18n({ useScope: 'global' });
  const theme = useTheme();
  const uiSettings = useUiSettings();
  const api = useApi();
  const connected = ref(side != 'remote');
  let unlisten: (() => void) | null = null;

  watch(
    () => uiSettings.settings.appearance,
    (newSettings, oldSettings) => {
      if (newSettings.language != oldSettings.language) {
        if (newSettings.language != null) {
          locale.value = newSettings.language;
        } else {
          locale.value = navigator.language;
        }
      }
      if (newSettings.darkMode != oldSettings.darkMode) {
        theme.change(newSettings.darkMode);
      }
    },
  );

  onMounted(() => {
    if (side == 'remote') {
      api.remote
        ?.onConnectionStatusChanged((isConnected) => {
          connected.value = isConnected;
        })
        .then((ulfn) => (unlisten = ulfn));
      api.remote?.isConnected().then((isConnected) => {
        connected.value = isConnected;
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
    overscroll-behavior-y: none;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }
  body,
  #app {
    height: 100%;
  }
</style>
