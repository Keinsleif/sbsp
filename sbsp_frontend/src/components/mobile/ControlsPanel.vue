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
import { mdiArrowDownRight, mdiArrowUpLeft, mdiFastForward, mdiPause, mdiPlay, mdiRepeat, mdiRewind, mdiSkipNext, mdiSkipPrevious, mdiStop } from '@mdi/js';
import SeekBar from './SeekBar.vue';
import { useUiSettings } from '../../stores/uiSettings';
import { useAssetResult } from '../../stores/assetResult';
import PathIcon from '../display/PathIcon.vue';
import ButtonGroup from 'primevue/buttongroup';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';

const api = useApi();
const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const showState = useShowState();
const uiSettings = useUiSettings();
const assetResult = useAssetResult();

const playbackCursorCue = computed(() => {
  return showState.playbackCursor != null ? getCueById.value(showState.playbackCursor) : null;
});

const playbackCursorCueDuration = computed(() => {
  return showState.playbackCursor != null ? assetResult.getMetadata(showState.playbackCursor)?.duration || null : null;
});

const activeTargetCue = computed(() => {
  if (showState.playbackCursor == null) return null;
  const activeCue = showState.activeCues[showState.playbackCursor];
  if (activeCue == null) return null;
  return activeCue;
});

const playbackCursorCueTitle = computed(() => {
  if (playbackCursorCue.value != null) {
    let text = '';
    if (playbackCursorCue.value.number.trim() !== '') {
      text = playbackCursorCue.value.number + '・';
    }
    text += playbackCursorCue.value.name != null ? playbackCursorCue.value.name : buildCueName(playbackCursorCue.value);
    return text;
  }
  return '';
});

const isCueStatus = computed(() => {
  if (showState.playbackCursor != null) {
    const activeCue = showState.activeCues[showState.playbackCursor];
    if (activeCue != null) {
      return (status: PlaybackStatus) => activeCue.status === status;
    }
  }
  return () => false;
});

const handleReadyPauseButton = () => {
  if (showState.playbackCursor != null) {
    switch (showState.activeCues[showState.playbackCursor]?.status) {
      case 'preWaiting':
      case 'playing': {
        api.sendPause(showState.playbackCursor);
        break;
      }
      case 'preWaitPaused':
      case 'paused': {
        api.sendResume(showState.playbackCursor);
        break;
      }
      case undefined: {
        api.sendLoad(showState.playbackCursor);
        break;
      }
    }
  }
};

const skipPrevious = () => {
  if (showState.playbackCursor != null) {
    let cursorIndex = showModel.flatCueList.findIndex(item => item.cue.id === showState.playbackCursor);
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
    api.setPlaybackCursor(cursorCueRef.cue.id);
  } else {
    const firstCueId = showModel.flatCueList[0]?.cue.id;
    if (firstCueId != null) {
      api.setPlaybackCursor(firstCueId);
    }
  }
};

const skipNext = () => {
  if (showState.playbackCursor != null) {
    let cursorIndex = showModel.flatCueList.findIndex(item => item.cue.id === showState.playbackCursor);
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
    api.setPlaybackCursor(cursorCueRef.cue.id);
  } else {
    const lastCueId = showModel.flatCueList[showModel.flatCueList.length - 1]?.cue.id;
    if (lastCueId != null) {
      api.setPlaybackCursor(lastCueId);
    }
  }
};

const skipToParent = () => {
  const cursorEntry = showModel.flatCueList.find(item => item.cue.id === showState.playbackCursor);
  if (cursorEntry == null || cursorEntry.parent == null) return;

  api.setPlaybackCursor(cursorEntry.parent);
};

const skipToChild = () => {
  const cursorEntry = showModel.flatCueList.find(item => item.cue.id === showState.playbackCursor);
  if (cursorEntry == null || cursorEntry.cue.params.type !== 'group') return;
  const firstChildId = cursorEntry.cue.params.children[0];
  if (firstChildId != null) {
    api.setPlaybackCursor(firstChildId);
  }
};

const rewind = () => {
  if (showState.playbackCursor != null && activeTargetCue.value != null) {
    api.sendSeekBy(showState.playbackCursor, -uiSettings.settings.general.seekAmount);
  }
};
const toggleRepeat = () => {
  if (showState.playbackCursor != null && activeTargetCue.value != null) {
    api.sendToggleRepeat(showState.playbackCursor);
  }
};
const fastForward = () => {
  if (showState.playbackCursor != null && activeTargetCue.value != null) {
    api.sendSeekBy(showState.playbackCursor, uiSettings.settings.general.seekAmount);
  }
};
</script>

<template>
  <div class="flex flex-col h-full p-3 gap-3">
    <h2
      class="p-1 mb-1 overflow-x-hidden h-9"
    >
      {{ playbackCursorCueTitle }}
    </h2>
    <div class="flex flex-row items-center gap-1">
      <path-icon :icon="playbackCursorCue != null ? getCueIcon(playbackCursorCue.params.type) : ''" />
      {{ playbackCursorCue != null ? firstUpper(playbackCursorCue.params.type) : '' }}
      <div class="ml-auto mr-0">
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

    <seek-bar :target-id="showState.playbackCursor" class="mt-auto" />
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
        :disabled="showState.playbackCursor == null"
        class="grow"
        severity="secondary"
        :blink="isCueStatus('stopping')"
        @click="
          if (showState.playbackCursor != null) {
            api.sendStop(showState.playbackCursor);
          }
        "
      />
      <button-wrapper
        :icon="mdiPlay"
        :active="isCueStatus('playing') || isCueStatus('preWaiting')"
        :disabled="showState.playbackCursor == null"
        active-color="green.500"
        class="grow"
        severity="secondary"
        :blink="isCueStatus('preWaiting')"
        @click="
          if (showState.playbackCursor != null) {
            if (isCueStatus('paused') || isCueStatus('preWaitPaused')) {
              api.sendResume(showState.playbackCursor);
            } else {
              api.sendGo();
            }
          }
        "
      />
      <button-wrapper
        :icon="mdiPause"
        :active="isCueStatus('paused') || isCueStatus('loaded')"
        :disabled="showState.playbackCursor == null"
        active-color="orange.500"
        class="grow"
        severity="secondary"
        :blink="isCueStatus('loaded')"
        @click="handleReadyPauseButton"
      />
    </button-group>
  </div>
</template>
