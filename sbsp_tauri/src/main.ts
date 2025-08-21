import { createApp } from 'vue';
import App from './App.vue';

// Vuetify
import 'vuetify/styles';
import { createVuetify } from 'vuetify';
import { aliases, mdi } from 'vuetify/iconsets/mdi-svg';
import { createPinia } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useShowModel } from './stores/showmodel';
import { useShowState } from './stores/showstate';
import { useUiState } from './stores/uistate';
import type { ShowState } from './types/ShowState';
import type { UiEvent } from './types/UiEvent';
import type { ShowModel } from './types/ShowModel';
import router from './router';

const vuetify = createVuetify({
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
  theme: {
    defaultTheme: 'system',
  },
});

const pinia = createPinia();

createApp(App).use(vuetify).use(router).use(pinia).mount('#app');

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();

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
        uiState.success('ShowModel loaded.');
      });
      break;
    case 'showModelSaved':
      uiState.success('ShowModel saved.');
      break;
    case 'cueUpdated':
      showModel.updateCue(event.payload.param.cue);
      break;
    case 'cueAdded':
      showModel.addCue(event.payload.param.cue, event.payload.param.atIndex);
      break;
    case 'cueRemoved':
      showModel.removeCue(event.payload.param.cueId);
      break;
    case 'cueMoved':
      showModel.moveCue(event.payload.param.cueId, event.payload.param.toIndex);
      break;
    case 'settingsUpdated': {
      const settings = event.payload.param.newSettings;
      showModel.$patch({ settings: settings });
      break;
    }
  }
});

invoke<ShowModel>('get_show_model')
  .then((model) => {
    showModel.updateAll(model);
  })
  .catch((e) => console.error(e.toString()));
