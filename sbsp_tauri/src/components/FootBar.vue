<template>
  <v-sheet class="d-flex align-center ml-0 mr-0 w-100">
    <v-sheet class="ml-0 mr-auto d-flex align-center">
      <!-- <v-switch
        inset
        hide-details
        color="primary"
        :true-icon="mdiLock"
        :false-icon="mdiLockOpen"
        v-model="showModel.settings.general.lockCursorToSelection"
      ></v-switch> -->
      <v-spacer></v-spacer>
      <v-btn :icon="mdiDockTop" size="small" variant="text"></v-btn>
    </v-sheet>
    <v-sheet class="ml-auto mr-auto"> {{ showModel.cues.length }} cues </v-sheet>
    <v-sheet class="mr-0 ml-auto d-flex align-center">
      <v-btn :icon="mdiDockBottom" size="small" variant="text" @click="uiState.toggleEditor"></v-btn>
      <v-btn :icon="mdiDockRight" size="small" variant="text" @click="uiState.toggleRightSidebar"></v-btn>
      <v-btn :icon="mdiCog" size="small" variant="text" @click="openSettings"></v-btn>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { mdiCog, mdiDockBottom, mdiDockRight, mdiDockTop } from '@mdi/js';
import { useUiState } from '../stores/uistate';
import { useShowModel } from '../stores/showmodel';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Window } from '@tauri-apps/api/window';

const showModel = useShowModel();
const uiState = useUiState();

const openSettings = async () => {
  const existSettingsWindow = await Window.getByLabel('settings');
  if (existSettingsWindow != null) {
    await existSettingsWindow.setFocus();
    return;
  }
  const settingsWindow = new WebviewWindow('settings', {
    url: '/settings',
    title: 'Settings',
    width: 1280,
    height: 720,
    resizable: false,
    dragDropEnabled: false,
  });
  settingsWindow.once('tauri://error', (e) => {
    console.error(e.payload);
  });
};
</script>
