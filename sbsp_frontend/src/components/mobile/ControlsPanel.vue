<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed } from 'vue';
import { useShowModel } from '../../stores/showModel';
import { storeToRefs } from 'pinia';
import { useApi } from '../../api';
import { useShowState } from '../../stores/showState';
import { buildCueName, firstUpper, getCueIcon, secondsToFormat } from '../../utils';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import {
  mdiArrowDownRight,
  mdiArrowUpLeft,
  mdiFastForward,
  mdiPause,
  mdiPlay,
  mdiRepeat,
  mdiRewind,
  mdiSkipNext,
  mdiSkipPrevious,
  mdiStop,
} from '@mdi/js';
import SeekBar from './SeekBar.vue';
import { useUiSettings } from '../../stores/uiSettings';
import { useAssetResult } from '../../stores/assetResult';
import PathIcon from '../display/PathIcon.vue';
import ButtonGroup from 'primevue/buttongroup';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import { useUiState } from '@/stores/uiState.ts';

const api = useApi();
const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const showState = useShowState();
const uiState = useUiState();
const uiSettings = useUiSettings();
const assetResult = useAssetResult();

const playbackCursorCue = computed(() => {
  return uiState.playbackCursor != null ? getCueById.value(uiState.playbackCursor) : null;
});

const playbackCursorCueDuration = computed(() => {
  return uiState.playbackCursor != null
    ? assetResult.getMetadata(uiState.playbackCursor)?.duration || null
    : null;
});

const activeTargetCue = computed(() => {
  if (uiState.playbackCursor == null) return null;
  const activeCue = showState.activeCues[uiState.playbackCursor];
  if (activeCue == null) return null;
  return activeCue;
});

const playbackCursorCueTitle = computed(() => {
  if (playbackCursorCue.value != null) {
    let text = '';
    if (playbackCursorCue.value.number.trim() !== '') {
      text = playbackCursorCue.value.number + '・';
    }
    text +=
      playbackCursorCue.value.name != null
        ? playbackCursorCue.value.name
        : buildCueName(playbackCursorCue.value);
    return text;
  }
  return '';
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

const skipPrevious = () => {
  if (uiState.playbackCursor != null) {
    let cursorIndex = showModel.flatCueList.findIndex(
      (item) => item.cue.id === uiState.playbackCursor,
    );
    if (cursorIndex < 0) return;
    const currentLevel = showModel.flatCueList[cursorIndex]!.level;

    cursorIndex--;
    let cursorCueRef = showModel.flatCueList[cursorIndex];
    if (cursorCueRef == null) return;

    while (cursorCueRef.level !== currentLevel) {
      cursorIndex--;
      cursorCueRef = showModel.flatCueList[cursorIndex];
      if (cursorCueRef == null) {
        return;
      }
    }
    uiState.setPlaybackCursor(cursorCueRef.cue.id);
  } else {
    const firstCueId = showModel.flatCueList[0]?.cue.id;
    if (firstCueId != null) {
      uiState.setPlaybackCursor(firstCueId);
    }
  }
};

const skipNext = () => {
  if (uiState.playbackCursor != null) {
    let cursorIndex = showModel.flatCueList.findIndex(
      (item) => item.cue.id === uiState.playbackCursor,
    );
    if (cursorIndex < 0) return;
    const currentLevel = showModel.flatCueList[cursorIndex]!.level;

    cursorIndex++;
    let cursorCueRef = showModel.flatCueList[cursorIndex];
    if (cursorCueRef == null) return;

    while (cursorCueRef.level !== currentLevel) {
      cursorIndex++;
      cursorCueRef = showModel.flatCueList[cursorIndex];
      if (cursorCueRef == null) {
        return;
      }
    }
    uiState.setPlaybackCursor(cursorCueRef.cue.id);
  } else {
    const lastCueId = showModel.flatCueList[showModel.flatCueList.length - 1]?.cue.id;
    if (lastCueId != null) {
      uiState.setPlaybackCursor(lastCueId);
    }
  }
};

const skipToParent = () => {
  const cursorEntry = showModel.flatCueList.find((item) => item.cue.id === uiState.playbackCursor);
  if (cursorEntry == null || cursorEntry.parent == null) return;

  uiState.setPlaybackCursor(cursorEntry.parent);
};

const skipToChild = () => {
  const cursorEntry = showModel.flatCueList.find((item) => item.cue.id === uiState.playbackCursor);
  if (cursorEntry == null || cursorEntry.cue.params.type !== 'group') return;
  const firstChildId = cursorEntry.cue.params.children[0];
  if (firstChildId != null) {
    uiState.setPlaybackCursor(firstChildId);
  }
};

const rewind = () => {
  if (uiState.playbackCursor != null && activeTargetCue.value != null) {
    api.sendSeekBy(uiState.playbackCursor, -uiSettings.settings.general.seekAmount);
  }
};
const toggleRepeat = () => {
  if (uiState.playbackCursor != null && activeTargetCue.value != null) {
    api.sendToggleRepeat(uiState.playbackCursor);
  }
};
const fastForward = () => {
  if (uiState.playbackCursor != null && activeTargetCue.value != null) {
    api.sendSeekBy(uiState.playbackCursor, uiSettings.settings.general.seekAmount);
  }
};
</script>

<template>
  <div class="flex h-full flex-col gap-3 p-3">
    <h2 class="mb-1 h-9 overflow-x-hidden p-1">
      {{ playbackCursorCueTitle }}
    </h2>
    <div class="flex flex-row items-center gap-1">
      <path-icon
        :icon="playbackCursorCue != null ? getCueIcon(playbackCursorCue.params.type) : ''"
      />
      {{ playbackCursorCue != null ? firstUpper(playbackCursorCue.params.type) : '' }}
      <div class="mr-0 ml-auto">
        {{ secondsToFormat(playbackCursorCueDuration) }}
      </div>
    </div>
    <button-group>
      <button-wrapper
        :icon="mdiArrowUpLeft"
        class="grow-2"
        severity="secondary"
        @click="skipToParent"
      />
      <button-wrapper
        :icon="mdiSkipPrevious"
        class="grow"
        severity="secondary"
        @click="skipPrevious"
      />
      <button-wrapper
        :icon="mdiSkipNext"
        class="grow"
        severity="secondary"
        @click="skipNext"
      />
      <button-wrapper
        :icon="mdiArrowDownRight"
        class="grow-2"
        severity="secondary"
        @click="skipToChild"
      />
    </button-group>

    <seek-bar
      :target-id="uiState.playbackCursor"
      class="mt-auto"
    />
    <button-group>
      <button-wrapper
        :icon="mdiRewind"
        :disabled="activeTargetCue == null"
        class="grow"
        severity="secondary"
        @click="rewind"
      />
      <button-wrapper
        :icon="mdiRepeat"
        :disabled="activeTargetCue == null"
        :active="activeTargetCue?.params.type == 'audio' && activeTargetCue.params.repeating"
        active-color="yellow.600"
        class="grow"
        severity="secondary"
        @click="toggleRepeat"
      />
      <button-wrapper
        :icon="mdiFastForward"
        :disabled="activeTargetCue == null"
        class="grow"
        severity="secondary"
        @click="fastForward"
      />
    </button-group>

    <button-group class="mb-0">
      <button-wrapper
        :icon="mdiStop"
        :active="isCueStatus('stopping')"
        active-color="red.500"
        :disabled="uiState.playbackCursor == null"
        class="grow"
        severity="secondary"
        :blink="isCueStatus('stopping')"
        @click="
          if (uiState.playbackCursor != null) {
            api.sendStop(uiState.playbackCursor);
          }
        "
      />
      <button-wrapper
        :icon="mdiPlay"
        :active="isCueStatus('playing') || isCueStatus('preWaiting')"
        :disabled="uiState.playbackCursor == null"
        active-color="green.500"
        class="grow"
        severity="secondary"
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
        :active="isCueStatus('paused') || isCueStatus('loaded')"
        :disabled="uiState.playbackCursor == null"
        active-color="orange.500"
        class="grow"
        severity="secondary"
        :blink="isCueStatus('loaded')"
        @click="handleReadyPauseButton"
      />
    </button-group>
  </div>
</template>
