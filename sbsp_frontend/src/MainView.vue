<template>
  <v-app height="100vh">
    <v-app-bar app border flat height="200">
      <ToolHeader />
    </v-app-bar>

    <v-main style="height: 100vh">
      <v-sheet class="d-flex fill-height list-wrapper">
        <CueList />
      </v-sheet>
    </v-main>

    <v-footer app border class="py-1">
      <FootBar />
    </v-footer>

    <v-navigation-drawer v-model="uiState.isRightSidebarOpen" app permanent location="right" width="300">
      <SideBar />
    </v-navigation-drawer>

    <v-navigation-drawer v-model="uiState.isEditorOpen" app permanent location="bottom" width="302">
      <BottomEditor v-model="selectedCue" @update="onCueEdited" />
    </v-navigation-drawer>

    <v-snackbar-queue v-model="uiState.success_messages" timeout="2000" color="success"></v-snackbar-queue>
    <v-snackbar-queue v-model="uiState.error_messages" timeout="2000" color="error"></v-snackbar-queue>

    <renumber-dialog v-model="uiState.isRenumberCueDialogOpen"></renumber-dialog>
    <settings-dialog v-model="uiState.isSettingsDialogOpen"></settings-dialog>
    <update-dialog v-model="uiState.isUpdateDialogOpen"></update-dialog>
  </v-app>
</template>

<script setup lang="ts">
import { useHotkey } from 'vuetify';
import { invoke } from '@tauri-apps/api/core';
import ToolHeader from './components/ToolHeader.vue';
import CueList from './components/CueList.vue';
import SideBar from './components/SideBar.vue';
import FootBar from './components/FootBar.vue';
import BottomEditor from './components/BottomEditor.vue';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';
import { computed, onMounted, onUnmounted, ref, toRaw, watch } from 'vue';
import { useShowState } from './stores/showstate';
import { listen } from '@tauri-apps/api/event';
import RenumberDialog from './components/dialog/RenumberDialog.vue';
import { PlaybackStatus } from './types/PlaybackStatus';
import SettingsDialog from './components/dialog/SettingsDialog.vue';
import type { Cue } from './types/Cue';
import { debounce } from 'vuetify/lib/util/helpers.mjs';
import { useI18n } from 'vue-i18n';
import type { ShowState } from './types/ShowState';
import type { UiEvent } from './types/UiEvent';
import type { ShowModel } from './types/ShowModel';
import UpdateDialog from './components/dialog/UpdateDialog.vue';
import { useAssetResult } from './stores/assetResult';
import { useUiSettings } from './stores/uiSettings';
import { getLockCursorToSelection } from './utils';
import { menu } from './window-menu';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
const assetResult = useAssetResult();
const uiSettings = useUiSettings();
const { t } = useI18n();

listen<ShowState>('backend-state-update', (event) => {
  showState.update(event.payload);
});

listen<UiEvent>('backend-event', (event) => {
  switch (event.payload.type) {
    case 'playbackCursorMoved': {
      if (getLockCursorToSelection()) {
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
      getCurrentWebviewWindow().setTitle(
        (uiState.side == 'main' ? 'SBS Player - ' : 'SBSP Remote - ') + showModel.name,
      );
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
    case 'modelNameUpdated':
      showModel.$patch({ name: event.payload.param.newName });
      getCurrentWebviewWindow().setTitle(
        (uiState.side == 'main' ? 'SBS Player - ' : 'SBSP Remote - ') + showModel.name,
      );
      break;
    case 'settingsUpdated': {
      const settings = event.payload.param.newSettings;
      showModel.$patch({ settings: settings });
      break;
    }
    case 'assetResult': {
      if ('Ok' in event.payload.param.data) {
        assetResult.add(event.payload.param.path, event.payload.param.data.Ok);
      } else {
        console.error(event.payload.param.data.Err);
        uiState.error(event.payload.param.data.Err);
      }
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

const wakeLock = ref<WakeLockSentinel | null>(null);

const onVisibilityChange = () => {
  if (wakeLock.value !== null && document.visibilityState === 'visible') {
    navigator.wakeLock.request('screen').then((value) => {
      wakeLock.value = value;
    });
  }
};

onMounted(() => {
  menu.setAsWindowMenu();
  invoke<'remote' | 'main'>('get_side').then((side) => {
    uiState.side = side;
    getCurrentWebviewWindow().setTitle((side == 'main' ? 'SBS Player - ' : 'SBSP Remote - ') + showModel.name);
  });
  navigator.wakeLock.request('screen').then((value) => {
    wakeLock.value = value;
  });
  document.addEventListener('visibilitychange', onVisibilityChange);
});

onUnmounted(() => {
  document.removeEventListener('visibilitychange', onVisibilityChange);
  if (wakeLock.value != null) {
    wakeLock.value.release().then(() => {
      wakeLock.value = null;
    });
  }
});

const selectedCue = ref<Cue | null>(
  uiState.selected != null ? showModel.cues.find((cue) => cue.id == uiState.selected)! : null,
);

watch(
  () => uiState.selected,
  () => {
    selectedCue.value = uiState.selected != null ? showModel.cues.find((cue) => cue.id == uiState.selected)! : null;
  },
);

const onCueEdited = debounce(() => {
  if (selectedCue.value == null) {
    return;
  }
  invoke('update_cue', { cue: toRaw(selectedCue.value) });
}, 500);

useHotkey(
  'cmd+o',
  () => {
    invoke('file_open');
  },
  { preventDefault: true },
);

useHotkey(
  'cmd+s',
  () => {
    invoke('file_save');
  },
  { preventDefault: true },
);

useHotkey(
  'cmd+shift+a',
  () => {
    invoke('file_save_as');
  },
  { preventDefault: true },
);

const goHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.go != null ? uiSettings.settings.hotkey.playback.go : undefined,
);
const loadHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.load != null ? uiSettings.settings.hotkey.playback.load : undefined,
);
const pauseAndResumeHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.pauseAndResume != null
    ? uiSettings.settings.hotkey.playback.pauseAndResume
    : undefined,
);
const pauseAllHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.pauseAll != null ? uiSettings.settings.hotkey.playback.pauseAll : undefined,
);
const resumeAllHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.resumeAll != null ? uiSettings.settings.hotkey.playback.resumeAll : undefined,
);
const stopHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.stop != null ? uiSettings.settings.hotkey.playback.stop : undefined,
);
const stopAllHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.stopAll != null ? uiSettings.settings.hotkey.playback.stopAll : undefined,
);
const seekForwardHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.seekForward != null ? uiSettings.settings.hotkey.playback.seekForward : undefined,
);
const seekBackwardHotkey = computed(() =>
  uiSettings.settings.hotkey.playback.seekBackward != null
    ? uiSettings.settings.hotkey.playback.seekBackward
    : undefined,
);
const audioToggleRepeatHotkey = computed(() =>
  uiSettings.settings.hotkey.audioAction.toggleRepeat != null
    ? uiSettings.settings.hotkey.audioAction.toggleRepeat
    : undefined,
);

useHotkey(
  goHotkey,
  () => {
    invoke('go').catch((e) => console.error(e));
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  loadHotkey,
  () => {
    for (let cueId of uiState.selectedRows) {
      invoke('load', { cueId: cueId }).catch((e) => console.error(e));
    }
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  pauseAndResumeHotkey,
  () => {
    if (uiState.selected != null && uiState.selected in showState.activeCues) {
      if ((['PreWaiting', 'Playing'] as PlaybackStatus[]).includes(showState.activeCues[uiState.selected]!.status)) {
        invoke('pause', { cueId: uiState.selected }).catch((e) => console.error(e));
      } else if (
        (['PreWaitPaused', 'Paused'] as PlaybackStatus[]).includes(showState.activeCues[uiState.selected]!.status)
      ) {
        invoke('resume', { cueId: uiState.selected }).catch((e) => console.error(e));
      }
    }
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  pauseAllHotkey,
  () => {
    invoke('pause_all').catch((e) => console.error(e));
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  resumeAllHotkey,
  () => {
    invoke('resume_all').catch((e) => console.error(e));
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  stopHotkey,
  () => {
    for (let cueId of uiState.selectedRows) {
      invoke('stop', { cueId: cueId }).catch((e) => console.error(e));
    }
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  stopAllHotkey,
  () => {
    invoke('stop_all').catch((e) => console.error(e));
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  seekForwardHotkey,
  () => {
    if (uiState.selected != null && uiState.selected in showState.activeCues) {
      if (
        !(['Loaded', 'Completed', 'Stopped', 'Error'] as PlaybackStatus[]).includes(
          showState.activeCues[uiState.selected]!.status,
        )
      ) {
        invoke('seek_by', { cueId: uiState.selected, amount: uiSettings.settings.general.seekAmount }).catch((e) =>
          console.error(e),
        );
      }
    }
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  seekBackwardHotkey,
  () => {
    if (uiState.selected != null && uiState.selected in showState.activeCues) {
      if (
        !(['Loaded', 'Completed', 'Stopped', 'Error'] as PlaybackStatus[]).includes(
          showState.activeCues[uiState.selected]!.status,
        )
      ) {
        invoke('seek_by', { cueId: uiState.selected, amount: -uiSettings.settings.general.seekAmount }).catch((e) =>
          console.error(e),
        );
      }
    }
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  audioToggleRepeatHotkey,
  () => {
    for (let cueId of uiState.selectedRows) {
      invoke('toggle_repeat', { cueId: cueId }).catch((e) => console.log(e));
    }
  },
  {
    preventDefault: true,
  },
);

useHotkey(
  'cmd+r',
  () => {
    uiState.isRenumberCueDialogOpen = true;
  },
  { preventDefault: true },
);
</script>
