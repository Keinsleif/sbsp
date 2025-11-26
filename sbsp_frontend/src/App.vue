<template>
  <router-view />
</template>

<script setup lang="ts">
import { watch } from 'vue';
import { useUiSettings } from './stores/uiSettings';
import { useI18n } from 'vue-i18n';
import { useTheme } from 'vuetify';

const { locale } = useI18n({ useScope: 'global' });
const theme = useTheme();
const uiSettings = useUiSettings();

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
