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
      <BottomEditor :selected-id="uiState.selected" />
    </v-navigation-drawer>

    <v-snackbar-queue v-model="uiState.success_messages" timeout="2000" color="success"></v-snackbar-queue>
    <v-snackbar-queue v-model="uiState.error_messages" timeout="2000" color="error"></v-snackbar-queue>

    <renumber-dialog v-model="isRenumberDialogOpen"></renumber-dialog>
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
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useShowState } from './stores/showstate';
import { listen } from '@tauri-apps/api/event';
import RenumberDialog from './components/dialog/RenumberDialog.vue';
import { PlaybackStatus } from './types/PlaybackStatus';

const uiState = useUiState();
const showState = useShowState();
const showModel = useShowModel();

const isRenumberDialogOpen = ref(false);

const wakeLock = ref<WakeLockSentinel | null>(null);

const onVisibilityChange = () => {
  if (wakeLock.value !== null && document.visibilityState === 'visible') {
    navigator.wakeLock.request('screen').then((value) => {
      wakeLock.value = value;
    });
  }
};

onMounted(() => {
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
  showModel.settings.hotkey.playback.go != null ? showModel.settings.hotkey.playback.go : undefined,
);
const loadHotkey = computed(() =>
  showModel.settings.hotkey.playback.load != null ? showModel.settings.hotkey.playback.load : undefined,
);
const pauseAndResumeHotkey = computed(() =>
  showModel.settings.hotkey.playback.pauseAndResume != null
    ? showModel.settings.hotkey.playback.pauseAndResume
    : undefined,
);
const pauseAllHotkey = computed(() =>
  showModel.settings.hotkey.playback.pauseAll != null ? showModel.settings.hotkey.playback.pauseAll : undefined,
);
const resumeAllHotkey = computed(() =>
  showModel.settings.hotkey.playback.resumeAll != null ? showModel.settings.hotkey.playback.resumeAll : undefined,
);
const stopHotkey = computed(() =>
  showModel.settings.hotkey.playback.stop != null ? showModel.settings.hotkey.playback.stop : undefined,
);
const stopAllHotkey = computed(() =>
  showModel.settings.hotkey.playback.stopAll != null ? showModel.settings.hotkey.playback.stopAll : undefined,
);
const audioToggleRepeatHotkey = computed(() =>
  showModel.settings.hotkey.audioAction.toggleRepeat != null
    ? showModel.settings.hotkey.audioAction.toggleRepeat
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

listen<string>('menu_clicked', (event) => {
  switch (event.payload) {
    case 'id_delete':
      for (const row of uiState.selectedRows) {
        invoke('remove_cue', { cueId: row }).catch((e) => console.error(e));
      }
      break;
    case 'id_renumber':
      isRenumberDialogOpen.value = true;
      break;
  }
});
useHotkey(
  'cmd+r',
  () => {
    isRenumberDialogOpen.value = true;
  },
  { preventDefault: true },
);
</script>
