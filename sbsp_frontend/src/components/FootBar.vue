<template>
  <v-sheet class="d-flex align-center ml-0 mr-0 w-100">
    <v-sheet class="ml-0 mr-auto d-flex align-center">
      <v-switch
        hide-details
        :true-icon="mdiPencil"
        :false-icon="mdiEye"
        v-model="uiState.mode"
        true-value="edit"
        false-value="run"
        density="compact"
      ></v-switch>
    </v-sheet>
    <v-sheet class="ml-auto mr-auto"> {{ showModel.cueCount }} {{ t('main.footBar.cueCountSuffix') }} </v-sheet>
    <v-sheet class="mr-0 ml-auto d-flex align-center">
      <v-btn
        v-if="uiState.side == 'main'"
        :icon="mdiServer"
        size="small"
        variant="text"
        @click="openServerPanel"
      ></v-btn>
      <v-btn :icon="mdiDockBottom" size="small" variant="text" @click="uiState.toggleBottomTab"></v-btn>
      <v-btn :icon="mdiDockRight" size="small" variant="text" @click="uiState.toggleRightSidebar"></v-btn>
      <v-btn :icon="mdiCog" size="small" variant="text" @click="openSettings"></v-btn>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { mdiCog, mdiDockBottom, mdiDockRight, mdiEye, mdiPencil, mdiServer } from '@mdi/js';
import { useUiState } from '../stores/uistate';
import { useShowModel } from '../stores/showmodel';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { message } from '@tauri-apps/plugin-dialog';

const { t } = useI18n();

const showModel = useShowModel();
const uiState = useUiState();

const openSettings = async () => {
  uiState.isSettingsDialogOpen = true;
};

const openServerPanel = () => {
  invoke('open_server_panel').catch((e) => {
    if (e[0]) {
      message(t('dialog.message.license.serverPanel'), { title: t('dialog.message.license.proTitle') });
    } else {
      console.error(e);
    }
  });
};
</script>
