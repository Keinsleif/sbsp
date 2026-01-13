<template>
  <component :is="connected ? MainView : ConnectView" />
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { useUiSettings } from './stores/uiSettings';
import { useI18n } from 'vue-i18n';
import { useTheme } from 'vuetify';
import { useApi } from './api';
import MainView from './MainView.vue';
import ConnectView from './ConnectView.vue';

const { locale } = useI18n({ useScope: 'global' });
const theme = useTheme();
const uiSettings = useUiSettings();
const api = useApi();
const connected = ref(api.side != 'remote');
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
  if (api.side == 'remote') {
    const searchParams = new URLSearchParams(window.location.search);
    const address = searchParams.get('address');
    if (address != null) {
      console.log(`Connecting to ${address}`);
      api.remote?.connectToServer(address);
    }
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
}
body,
#app {
  height: 100%;
}
</style>
