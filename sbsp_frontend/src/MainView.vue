<template>
  <v-app height="100vh" @contextmenu.prevent>
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

    <v-navigation-drawer
      :model-value="uiState.isRightSidebarOpen && mdAndUp"
      app
      permanent
      persistent
      touchless
      location="right"
      width="260"
    >
      <SideBar />
    </v-navigation-drawer>

    <v-navigation-drawer
      :model-value="uiState.isBottomTabOpen && uiState.mode == 'edit'"
      app
      permanent
      persistent
      touchless
      location="bottom"
      width="250"
    >
      <BottomEditor v-model="selectedCue" @update="onCueEdited" :sequence-override="selectedCueSequenceOverride" />
    </v-navigation-drawer>

    <v-snackbar-queue v-model="uiState.success_messages" timeout="2000" color="success"></v-snackbar-queue>
    <v-snackbar-queue v-model="uiState.error_messages" timeout="2000" color="error"></v-snackbar-queue>

    <renumber-dialog v-model="uiState.isRenumberCueDialogOpen"></renumber-dialog>
    <settings-dialog v-model="uiState.isSettingsDialogOpen"></settings-dialog>
    <file-list-dialog
      v-if="side == 'remote'"
      v-model="uiState.fileListResolver"
      :multiple="uiState.fileListOption"
    ></file-list-dialog>
    <server-panel-dialog v-if="side == 'host'" v-model="uiState.isServerPanelOpen"></server-panel-dialog>
  </v-app>
</template>

<script setup lang="ts">
  import { useHotkey } from 'vuetify';
  import ToolHeader from './components/ToolHeader.vue';
  import CueList from './components/CueList.vue';
  import SideBar from './components/SideBar.vue';
  import FootBar from './components/FootBar.vue';
  import BottomEditor from './components/BottomEditor.vue';
  import { useUiState } from './stores/uistate';
  import { useShowModel } from './stores/showmodel';
  import { computed, onMounted, onUnmounted, ref, toRaw, watch } from 'vue';
  import { useShowState } from './stores/showstate';
  import RenumberDialog from './components/dialog/RenumberDialog.vue';
  import { PlaybackStatus } from './types/PlaybackStatus';
  import SettingsDialog from './components/dialog/SettingsDialog.vue';
  import type { Cue } from './types/Cue';
  import { debounce } from './utils';
  import { useI18n } from 'vue-i18n';
  import type { ShowState } from './types/ShowState';
  import { useAssetResult } from './stores/assetResult';
  import { useUiSettings } from './stores/uiSettings';
  import { getLockCursorToSelection } from './utils';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import FileListDialog from './components/dialog/FileListDialog.vue';
  import ServerPanelDialog from './components/dialog/ServerPanelDialog.vue';
  import { message } from '@tauri-apps/plugin-dialog';
  import { useApi, side } from './api';
  import { useDisplay } from 'vuetify/lib/composables/display.mjs';

  const showModel = useShowModel();
  const showState = useShowState();
  const uiState = useUiState();
  const assetResult = useAssetResult();
  const uiSettings = useUiSettings();
  const { t } = useI18n();
  const api = useApi();
  const { mdAndUp } = useDisplay();

  const wakeLock = ref<WakeLockSentinel | null>(null);

  const onVisibilityChange = () => {
    if (wakeLock.value !== null && document.visibilityState === 'visible') {
      navigator.wakeLock.request('screen').then((value) => {
        wakeLock.value = value;
      });
    }
  };

  const unlistenFuncs: (() => void)[] = [];
  let rafNumber: number | null = null;

  onMounted(() => {
    api.setTitle((side == 'host' ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);

    let latestState: ShowState | null = null;
    api
      .onStateUpdate((state) => {
        latestState = state;
      })
      .then((unlistenFn) => unlistenFuncs.push(unlistenFn));
    const updateLoop = () => {
      if (latestState != null) {
        showState.update(latestState);
        latestState = null;
      }
      rafNumber = requestAnimationFrame(updateLoop);
    };
    rafNumber = requestAnimationFrame(updateLoop);

    api
      .onUiEvent((event) => {
        switch (event.type) {
          case 'playbackCursorMoved': {
            if (getLockCursorToSelection()) {
              const cueId = event.param.cueId;
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
            showModel.updateAll(event.param.model);
            uiState.success(t('notification.modelLoaded'));
            api.setTitle((side == 'host' ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);
            break;
          case 'showModelSaved':
            uiState.success(t('notification.modelSaved'));
            break;
          case 'showModelReset':
            showModel.updateAll(event.param.model);
            api.setTitle((side == 'host' ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);
            break;
          case 'cueRemoved':
            if (uiState.selectedRows.includes(event.param.cueId)) {
              uiState.removeFromSelected(event.param.cueId);
            }
            break;
          case 'cueListUpdated':
            showModel.$patch({ cues: event.param.cues });
            break;
          case 'modelNameUpdated':
            showModel.$patch({ name: event.param.newName });
            api.setTitle((side == 'host' ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);
            break;
          case 'settingsUpdated': {
            const settings = event.param.newSettings;
            showModel.$patch({ settings: settings });
            break;
          }
          case 'assetResult': {
            if ('Ok' in event.param.data) {
              assetResult.add(event.param.path, event.param.data.Ok);
            } else {
              console.error(event.param.data.Err);
              uiState.error(event.param.data.Err);
            }
            break;
          }
          case 'cueError':
            console.error(event.param.error);
            uiState.error(event.param.error);
            break;
          case 'operationFailed':
            console.error(event.param.error);
            uiState.error(event.param.error.message);
            break;
        }
      })
      .then((unlistenFn) => unlistenFuncs.push(unlistenFn));

    if (side == 'host') {
      getCurrentWebviewWindow()
        .onCloseRequested(async (event) => {
          const isModified = await api.isModified();
          if (isModified) {
            const result = await message(t('dialog.saveConfirm.content'), {
              buttons: {
                yes: t('dialog.saveConfirm.save'),
                no: t('dialog.saveConfirm.dontSave'),
                cancel: t('dialog.saveConfirm.cancel'),
              },
            }).catch((e) => console.error(e));
            switch (result) {
              case t('dialog.saveConfirm.save'):
                await api.host?.fileSave();
                break;
              case t('dialog.saveConfirm.dontSave'):
                break;
              case t('dialog.saveConfirm.cancel'):
                event.preventDefault();
                break;
            }
          }
        })
        .then((unlistenFn) => unlistenFuncs.push(unlistenFn));
    }
    api
      .getShowModel()
      .then((model) => {
        showModel.updateAll(model);
      })
      .catch((e) => console.error(e.toString()));

    if (navigator.wakeLock) {
      navigator.wakeLock
        .request('screen')
        .then((value) => {
          wakeLock.value = value;
        })
        .catch((e) => console.error(e));
    }
    document.addEventListener('visibilitychange', onVisibilityChange);
  });

  onUnmounted(() => {
    unlistenFuncs.forEach((unlistenFn) => unlistenFn());
    document.removeEventListener('visibilitychange', onVisibilityChange);
    if (wakeLock.value != null) {
      wakeLock.value
        .release()
        .then(() => {
          wakeLock.value = null;
        })
        .catch((e) => console.error(e));
    }
    if (rafNumber != null) {
      cancelAnimationFrame(rafNumber);
    }
  });

  const selectedCue = ref<Cue | null>(uiState.selected != null ? showModel.getCueById(uiState.selected)! : null);
  const selectedCueSequenceOverride = computed(() => {
    if (selectedCue.value == null) {
      return null;
    }
    const flatEntry = showModel.flatCueList.find((item) => item.cue.id == selectedCue.value!.id);
    if (flatEntry == null) {
      return null;
    }
    if (flatEntry.isSequenceOverrided) {
      return flatEntry.sequence;
    } else {
      return null;
    }
  });

  watch(
    () => uiState.selected,
    () => {
      if (onCueEdited.debouncing) {
        onCueEdited.clear();
        onCueEdited.immediate();
      }
      selectedCue.value = uiState.selected != null ? showModel.getCueById(uiState.selected)! : null;
    },
  );

  const onCueEdited = debounce(() => {
    if (selectedCue.value == null) {
      return;
    }
    api.updateCue(toRaw(selectedCue.value));
  }, 500);

  useHotkey(
    'cmd+o',
    () => {
      api.host?.fileOpen();
    },
    { preventDefault: true },
  );

  useHotkey(
    'cmd+s',
    () => {
      api.host?.fileSave();
    },
    { preventDefault: true },
  );

  useHotkey(
    'cmd+shift+a',
    () => {
      api.host?.fileSaveAs();
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
    uiSettings.settings.hotkey.playback.seekForward != null
      ? uiSettings.settings.hotkey.playback.seekForward
      : undefined,
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
      api.sendGo();
    },
    {
      preventDefault: true,
    },
  );

  useHotkey(
    loadHotkey,
    () => {
      for (let cueId of uiState.selectedRows) {
        api.sendLoad(cueId);
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
          api.sendPause(uiState.selected);
        } else if (
          (['PreWaitPaused', 'Paused'] as PlaybackStatus[]).includes(showState.activeCues[uiState.selected]!.status)
        ) {
          api.sendResume(uiState.selected);
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
      api.sendPauseAll();
    },
    {
      preventDefault: true,
    },
  );

  useHotkey(
    resumeAllHotkey,
    () => {
      api.sendResumeAll();
    },
    {
      preventDefault: true,
    },
  );

  useHotkey(
    stopHotkey,
    () => {
      for (let cueId of uiState.selectedRows) {
        api.sendStop(cueId);
      }
    },
    {
      preventDefault: true,
    },
  );

  useHotkey(
    stopAllHotkey,
    () => {
      api.sendStopAll();
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
          api.sendSeekBy(uiState.selected, uiSettings.settings.general.seekAmount);
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
          api.sendSeekBy(uiState.selected, -uiSettings.settings.general.seekAmount);
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
        api.sendToggleRepeat(cueId);
      }
    },
    {
      preventDefault: true,
    },
  );

  useHotkey(
    'cmd+r',
    () => {
      if (uiState.mode == 'edit') {
        uiState.isRenumberCueDialogOpen = true;
      }
    },
    { preventDefault: true },
  );
</script>
