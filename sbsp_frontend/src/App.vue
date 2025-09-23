<template>
  <router-view />
</template>

<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { useShowModel } from './stores/showmodel';
import { useShowState } from './stores/showstate';
import { useUiState } from './stores/uistate';
import type { ShowState } from './types/ShowState';
import type { UiEvent } from './types/UiEvent';
import type { ShowModel } from './types/ShowModel';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
const { t } = useI18n();

listen<ShowState>('backend-state-update', (event) => {
  showState.update(event.payload);
});

listen<UiEvent>('backend-event', (event) => {
  switch (event.payload.type) {
    case 'playbackCursorMoved': {
      if (showModel.settings.general.lockCursorToSelection) {
        const cueId = event.payload.param.cueId;
        if (cueId != null) {
          if (uiState.selected != cueId) {
            uiState.selected = cueId;
            if (!uiState.selectedRows.includes(cueId)) {
              uiState.selectedRows = [cueId];
            }
          }
        } else {
          uiState.selectedRows = [];
          uiState.selected = null;
        }
      }
      break;
    }
    case 'showModelLoaded':
      invoke<ShowModel>('get_show_model').then((model) => {
        showModel.updateAll(model);
        uiState.success(t('notification.modelLoaded'));
      });
      break;
    case 'showModelSaved':
      uiState.success(t('notification.modelSaved'));
      break;
    case 'cueUpdated':
      showModel.updateCue(event.payload.param.cue);
      break;
    case 'cueAdded':
      showModel.addCue(event.payload.param.cue, event.payload.param.atIndex);
      break;
    case 'cuesAdded':
      showModel.addCues(event.payload.param.cues, event.payload.param.atIndex);
      break;
    case 'cueRemoved':
      showModel.removeCue(event.payload.param.cueId);
      break;
    case 'cueMoved':
      showModel.moveCue(event.payload.param.cueId, event.payload.param.toIndex);
      break;
    case 'cueListUpdated':
      showModel.$patch({ cues: event.payload.param.cues });
      break;
    case 'settingsUpdated': {
      const settings = event.payload.param.newSettings;
      showModel.$patch({ settings: settings });
      break;
    }
    case 'operationFailed':
      console.error(event.payload.param.error);
      uiState.error(event.payload.param.error.message);
      break;
  }
});

invoke<ShowModel>('get_show_model')
  .then((model) => {
    showModel.updateAll(model);
  })
  .catch((e) => console.error(e.toString()));
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
