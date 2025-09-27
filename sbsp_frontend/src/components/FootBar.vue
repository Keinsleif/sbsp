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
    </v-sheet>
    <v-sheet class="ml-auto mr-auto"> {{ showModel.cues.length }} cues </v-sheet>
    <v-sheet class="mr-0 ml-auto d-flex align-center">
      <v-btn
        v-if="uiState.side == 'main'"
        :icon="mdiServer"
        size="small"
        variant="text"
        @click="openServerPanel"
      ></v-btn>
      <v-btn :icon="mdiDockBottom" size="small" variant="text" @click="uiState.toggleEditor"></v-btn>
      <v-btn :icon="mdiDockRight" size="small" variant="text" @click="uiState.toggleRightSidebar"></v-btn>
      <v-btn :icon="mdiCog" size="small" variant="text" @click="openSettings"></v-btn>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { mdiCog, mdiDockBottom, mdiDockRight, mdiServer } from '@mdi/js';
import { useUiState } from '../stores/uistate';
import { useShowModel } from '../stores/showmodel';
import { invoke } from '@tauri-apps/api/core';

const showModel = useShowModel();
const uiState = useUiState();

const openSettings = async () => {
  uiState.isSettingsDialogOpen = true;
};

const openServerPanel = () => {
  invoke('open_server_panel').catch((e) => console.error(e));
};
</script>
