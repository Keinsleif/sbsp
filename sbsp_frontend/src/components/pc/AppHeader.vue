<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import {
  mdiPause,
  mdiPlay,
  mdiStop,
  mdiChartBellCurveCumulative,
  mdiTimerSandEmpty,
  mdiVolumeHigh,
  mdiPlayCircleOutline,
  mdiStopCircleOutline,
  mdiPauseCircleOutline,
  mdiUploadCircleOutline,
  mdiGroup,
} from '@mdi/js';
import { useShowModel } from '../../stores/showModel.ts';
import { computed } from 'vue';
import { useShowState } from '../../stores/showState.ts';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { buildCueName } from '../../utils';
import { useNow, useWindowFocus } from '@vueuse/core';
import { useUiState } from '../../stores/uiState.ts';
import { useUiSettings } from '../../stores/uiSettings';
import { useApi } from '../../api';
import { storeToRefs } from 'pinia';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import ButtonGroup from 'primevue/buttongroup';

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const showState = useShowState();
const uiState = useUiState();
const uiSettings = useUiSettings();
const api = useApi();

const hasFocus = useWindowFocus();

const playbackCursorCue = computed(() => {
  return uiState.playbackCursor != null ? getCueById.value(uiState.playbackCursor) : null;
});

const playbackCursorCueTitle = computed(() => {
  return playbackCursorCue.value != null
    ? playbackCursorCue.value.number +
        '・' +
        (playbackCursorCue.value.name != null
          ? playbackCursorCue.value.name
          : buildCueName(playbackCursorCue.value))
    : ' ';
});

const isCueStatus = computed(() => {
  if (uiState.playbackCursor != null) {
    const activeCue = showState.activeCues[uiState.playbackCursor];
    if (activeCue != null) {
      return (status: PlaybackStatus) => activeCue.status === status;
    }
  }
  return () => false;
});

const handleReadyPauseButton = () => {
  if (uiState.playbackCursor != null) {
    switch (showState.activeCues[uiState.playbackCursor]?.status) {
      case 'preWaiting':
      case 'playing': {
        api.sendPause(uiState.playbackCursor);
        break;
      }
      case 'preWaitPaused':
      case 'paused': {
        api.sendResume(uiState.playbackCursor);
        break;
      }
      case undefined: {
        api.sendLoad(uiState.playbackCursor);
        break;
      }
    }
  }
};

const time = useNow();
</script>

<template>
  <div
    class="m-0 flex h-full w-full flex-row gap-3 overflow-x-auto border-b border-(--p-form-field-border-color) p-3"
  >
    <div class="flex grow-0 flex-col items-start gap-3">
      <div
        class="flex grow items-center border border-(--p-form-field-border-color)"
        :class="hasFocus ? '' : 'bg-red-500'"
      >
        <div class="flex items-end pr-3 pl-3 text-center text-6xl tabular-nums">
          <span>{{ String(time.getHours()).padStart(2, '0') }}</span
          >:<span>{{ String(time.getMinutes()).padStart(2, '0') }}</span
          >.<span class="text-4xl">{{ String(time.getSeconds()).padStart(2, '0') }}</span>
        </div>
      </div>
      <div
        :class="
          uiSettings.settings.appearance.hideControls || uiState.mode == 'view' ? 'hidden' : 'flex'
        "
        class="flex-col gap-3"
      >
        <button-group>
          <button-wrapper
            :icon="mdiStop"
            severity="secondary"
            :active="isCueStatus('stopping')"
            :blink="isCueStatus('stopping')"
            active-color="red.500"
            :disabled="uiState.playbackCursor == null"
            @click="
              if (uiState.playbackCursor != null) {
                api.sendStop(uiState.playbackCursor);
              }
            "
          />
          <button-wrapper
            :icon="mdiPlay"
            severity="secondary"
            :active="isCueStatus('playing') || isCueStatus('preWaiting')"
            :disabled="uiState.playbackCursor == null"
            active-color="green.600"
            :blink="isCueStatus('preWaiting')"
            @click="
              if (uiState.playbackCursor != null) {
                if (isCueStatus('paused') || isCueStatus('preWaitPaused')) {
                  api.sendResume(uiState.playbackCursor);
                } else {
                  api.sendExecute(uiState.playbackCursor);
                }
              }
            "
          />
          <button-wrapper
            :icon="mdiPause"
            severity="secondary"
            :active="isCueStatus('paused') || isCueStatus('loaded')"
            :disabled="uiState.playbackCursor == null"
            active-color="orange.600"
            :blink="isCueStatus('loaded')"
            @click="handleReadyPauseButton"
          />
        </button-group>
      </div>
    </div>
    <div class="flex grow flex-col gap-3">
      <div class="m-0 flex grow flex-col">
        <div
          class="mb-1 h-9 shrink-0 grow-0 overflow-x-hidden rounded border border-(--p-form-field-border-color) p-1 ps-3"
        >
          {{ playbackCursorCueTitle }}
        </div>
        <div
          class="grow overflow-y-auto rounded border border-(--p-form-field-border-color) p-2 pb-0 whitespace-pre-wrap"
        >
          {{ playbackCursorCue != null ? playbackCursorCue.notes : '' }}
        </div>
      </div>
      <div
        :class="uiState.mode == 'edit' ? 'flex' : 'hidden'"
        class="flex-row justify-end gap-4"
      >
        <button-group>
          <button-wrapper
            :icon="mdiVolumeHigh"
            severity="secondary"
            @click="showModel.addEmptyAudioCue()"
          />
          <button-wrapper
            :icon="mdiTimerSandEmpty"
            severity="secondary"
            @click="showModel.addEmptyWaitCue()"
          />
          <button-wrapper
            :icon="mdiChartBellCurveCumulative"
            severity="secondary"
            @click="showModel.addEmptyFadeCue()"
          />
        </button-group>
        <button-group>
          <button-wrapper
            :icon="mdiGroup"
            severity="secondary"
            @click="showModel.addEmptyGroupCue()"
          />
        </button-group>
        <button-group>
          <button-wrapper
            :icon="mdiPlayCircleOutline"
            severity="secondary"
            @click="showModel.addEmptyPlaybackCue('start')"
          />
          <button-wrapper
            :icon="mdiStopCircleOutline"
            severity="secondary"
            @click="showModel.addEmptyPlaybackCue('stop')"
          />
          <button-wrapper
            :icon="mdiPauseCircleOutline"
            severity="secondary"
            @click="showModel.addEmptyPlaybackCue('pause')"
          />
          <button-wrapper
            :icon="mdiUploadCircleOutline"
            severity="secondary"
            @click="showModel.addEmptyPlaybackCue('load')"
          />
        </button-group>
      </div>
    </div>
  </div>
</template>
