<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { breakpointsTailwind, useBreakpoints, useIntervalFn } from '@vueuse/core';
import MainViewDesktop from './MainViewDesktop.vue';
import MainViewMobile from './MainViewMobile.vue';
import { onMounted, onUnmounted, ref } from 'vue';
import { useShowModel } from './stores/showModel.ts';
import { useShowState } from './stores/showState.ts';
import { useUiState } from './stores/uiState.ts';
import { useAssetResult } from './stores/assetResult.ts';
import { useUiSettings } from './stores/uiSettings.ts';
import { useI18n } from 'vue-i18n';
import { useApi } from './api';
import { usePositionTicker } from './composables/usePosition.ts';
import { getLockCursorToSelection } from './utils.ts';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { message } from '@tauri-apps/plugin-dialog';
import { useHotkey } from './composables/useHotkey.ts';
import type { PlaybackStatus } from './types/PlaybackStatus.ts';

const breakpoints = useBreakpoints(breakpointsTailwind)
const xs = breakpoints.smaller('sm');

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
const assetResult = useAssetResult();
const uiSettings = useUiSettings();
const { t } = useI18n();
const api = useApi();

const wakeLock = ref<WakeLockSentinel | null>(null);

const onVisibilityChange = () => {
  if (wakeLock.value !== null && document.visibilityState === 'visible') {
    navigator.wakeLock.request('screen').then((value) => {
      wakeLock.value = value;
    });
  }
};

const unlistenFuncs: (() => void)[] = [];

onMounted(() => {
  document.addEventListener('touchmove', e => e.preventDefault(), { passive: false });
  api.setTitle((__IS_HOST__ ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);

  useIntervalFn(
    () => {
      api.requestStateSync();
    },
    5000,
    {
      immediate: true,
    },
  );

  usePositionTicker();

  api
    .onBackendEvent((event) => {
      switch (event.type) {
        case 'cueStatus':
          if (event.param.type === 'error') {
            console.error(event.param.error);
            uiState.error(event.param.error);
          }
          showState.handleCueStateEvent(event.param);
          break;
        case 'playbackCursorMoved': {
          showState.updatePlaybackCursor(event.param.cueId);
          if (getLockCursorToSelection()) {
            const cueId = event.param.cueId;
            if (cueId != null) {
              if (uiState.selected !== cueId) {
                uiState.selected = cueId;
                uiState.expandToVisible(cueId);
                // This operation not using uiState.addSelected to avoid updating playbackcursor.
                if (!uiState.selectedRows.has(cueId)) {
                  uiState.selectedRows.clear();
                  uiState.selectedRows.add(cueId);
                }
              }
            } else {
              // This operation not using uiState.addSelected to avoid updating playbackcursor.
              uiState.selectedRows.clear();
              uiState.selected = null;
            }
          }
          break;
        }
        case 'syncState':
          showState.handleSyncEvent(event.param);
          break;
        case 'showModelLoaded':
          showModel.updateAll(event.param.model);
          uiState.success(t('notification.modelLoaded'));
          api.setTitle((__IS_HOST__ ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);
          uiState.resetSelected();
          break;
        case 'showModelSaved':
          uiState.success(t('notification.modelSaved'));
          break;
        case 'showModelReset':
          showModel.updateAll(event.param.model);
          api.setTitle((__IS_HOST__ ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);
          uiState.resetSelected();
          break;
        case 'cueRemoved':
          uiState.removeFromSelected(event.param.cueIds);
          break;
        case 'cueListUpdated':
          showModel.$patch({ cues: event.param.cues, rootIds: event.param.rootIds });
          break;
        case 'modelNameUpdated':
          showModel.$patch({ name: event.param.newName });
          api.setTitle((__IS_HOST__ ? 'SBS Player - ' : 'SBS Player Remote - ') + showModel.name);
          break;
        case 'settingsUpdated': {
          const settings = event.param.newSettings;
          showModel.$patch({ settings: settings });
          break;
        }
        case 'assetMetadata': {
          assetResult.addMetadata(event.param.path, event.param.data);
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
        case 'operationFailed':
          console.error(event.param.error);
          switch (event.param.error.type) {
            case 'saveToFile':
              uiState.error(event.param.error.message);
              break;
            case 'loadFromFile':
              uiState.error(event.param.error.message);
              break;
            case 'exportToFolder':
              uiState.error(event.param.error.message);
              break;
            case 'cueEdit':
              uiState.error(event.param.error.message);
              break;
            case 'custom':
              switch (event.param.error.id) {
                case 1:
                  uiState.error(t('notification.authenticationFailed'));
                  break;
                case 2:
                  uiState.error(t('notification.permissionDenied'));
                  break;
                default:
                  uiState.error(event.param.error.message);
                  break;
              }
          }
          break;
      }
    })
    .then(unlistenFn => unlistenFuncs.push(unlistenFn));

  if (__IS_HOST__) {
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
            kind: 'warning',
            title: t('general.confirm'),
          }).catch(e => console.error(e));
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
      .then(unlistenFn => unlistenFuncs.push(unlistenFn));
  }
  api
    .getFullState()
    .then((fullState) => {
      showModel.updateAll(fullState.showModel);
      showState.update(fullState.showState);
      if (getLockCursorToSelection()) {
        const cueId = fullState.showState.playbackCursor;
        if (cueId != null) {
          if (uiState.selected !== cueId) {
            uiState.selected = cueId;
            uiState.expandToVisible(cueId);
            // This operation not using uiState.addSelected to avoid updating playbackcursor.
            if (!uiState.selectedRows.has(cueId)) {
              uiState.selectedRows.clear();
              uiState.selectedRows.add(cueId);
            }
          }
        } else {
          // This operation not using uiState.addSelected to avoid updating playbackcursor.
          uiState.selectedRows.clear();
          uiState.selected = null;
        }
      }
    })
    .catch(e => console.error(e.toString()));

  if (navigator.wakeLock) {
    navigator.wakeLock
      .request('screen')
      .then((value) => {
        wakeLock.value = value;
      })
      .catch(e => console.error(e));
  }
  document.addEventListener('visibilitychange', onVisibilityChange);
});

onUnmounted(() => {
  unlistenFuncs.forEach(unlistenFn => unlistenFn());
  document.removeEventListener('visibilitychange', onVisibilityChange);
  if (wakeLock.value != null) {
    wakeLock.value
      .release()
      .then(() => {
        wakeLock.value = null;
      })
      .catch(e => console.error(e));
  }
});

if (api.host) {
  useHotkey(
    'cmd+o',
    (e) => {
      e.preventDefault()
      api.host?.fileOpen();
    },
  );

  useHotkey(
    'cmd+s',
    (e) => {
      e.preventDefault();
      api.host?.fileSave();
    },
  );

  useHotkey(
    'cmd+shift+a',
    (e) => {
      e.preventDefault();
      api.host?.fileSaveAs();
    },
  );
}

useHotkey(
  uiSettings.settings.hotkey.playback.go,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      api.sendGo();
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.load,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      for (let cueId of uiState.selectedRows) {
        api.sendLoad(cueId);
      }
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.pauseAndResume,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view' && uiState.selected != null && uiState.selected in showState.activeCues) {
      if ((['preWaiting', 'playing'] as PlaybackStatus[]).includes(showState.activeCues[uiState.selected]!.status)) {
        api.sendPause(uiState.selected);
      } else if (
        (['preWaitPaused', 'paused'] as PlaybackStatus[]).includes(showState.activeCues[uiState.selected]!.status)
      ) {
        api.sendResume(uiState.selected);
      }
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.pauseAll,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      api.sendPauseAll();
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.resumeAll,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      api.sendResumeAll();
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.stop,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      for (let cueId of uiState.selectedRows) {
        api.sendStop(cueId);
      }
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.stopAll,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      api.sendStopAll();
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.seekForward,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view' && uiState.selected != null && uiState.selected in showState.activeCues) {
      api.sendSeekBy(uiState.selected, uiSettings.settings.general.seekAmount);
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.playback.seekBackward,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view' && uiState.selected != null && uiState.selected in showState.activeCues) {
      api.sendSeekBy(uiState.selected, -uiSettings.settings.general.seekAmount);
    }
  },
);

useHotkey(
  uiSettings.settings.hotkey.audioAction.toggleRepeat,
  (e) => {
    e.preventDefault();
    if (uiState.mode !== 'view') {
      for (let cueId of uiState.selectedRows) {
        api.sendToggleRepeat(cueId);
      }
    }
  },
);

useHotkey(
  'cmd+r',
  (e) => {
    e.preventDefault();
    if (uiState.mode === 'edit') {
      uiState.isRenumberCueDialogOpen = true;
    }
  },
);
</script>

<template>
  <component :is="xs ? MainViewMobile : MainViewDesktop" />
</template>