<template>
  <v-sheet class="d-flex flex-column h-100 pa-3 ga-3">
    <div
      class="pa-1 mb-1 overflow-x-hidden text-body-large font-weight-semibold"
      height="36px"
    >
      {{ playbackCursorCueTitle }}
    </div>
    <div class="d-flex">
      <v-icon :icon="playbackCursorCue != null ? getCueIcon(playbackCursorCue.params.type) : ''" />
      {{ playbackCursorCue != null ? firstUpper(playbackCursorCue.params.type) : '' }}
      <div class="ml-auto mr-0">
        {{ secondsToFormat(playbackCursorCueDuration) }}
      </div>
    </div>
    <v-btn-group
      divided
      variant="tonal"
    >
      <v-btn
        :icon="mdiArrowUpLeft"
        class="flex-grow-2 border-background"
        @click="skipToParent"
      />
      <v-btn
        :icon="mdiSkipPrevious"
        class="flex-grow-1 border-background"
        @click="skipPrevious"
      />
      <v-btn
        :icon="mdiSkipNext"
        class="flex-grow-1 border-background"
        @click="skipNext"
      />
      <v-btn
        :icon="mdiArrowDownRight"
        class="flex-grow-2 border-background"
        @click="skipToChild"
      />
    </v-btn-group>

    <seek-bar :target-id="showState.playbackCursor" class="mt-auto" />
    <v-btn-group
      divided
      variant="tonal"
    >
      <v-btn
        :icon="mdiRewind"
        :disabled="activeTargetCue == null"
        class="flex-grow-1 border-background"
        @click="rewind"
      />
      <v-btn
        :icon="mdiRepeat"
        :disabled="activeTargetCue == null"
        :active="activeTargetCue?.params.type == 'audio' && activeTargetCue.params.repeating"
        active-color="yellow"
        class="flex-grow-1 border-background"
        @click="toggleRepeat"
      />
      <v-btn
        :icon="mdiFastForward"
        :disabled="activeTargetCue == null"
        class="flex-grow-1 border-background"
        @click="fastForward"
      />
    </v-btn-group>

    <v-btn-group
      divided
      variant="tonal"
      class="mb-0"
    >
      <v-btn
        :icon="mdiStop"
        :active="isCueStatus('stopping')"
        active-color="red"
        :disabled="showState.playbackCursor == null"
        class="flex-grow-1 border-background"
        :class="[isCueStatus('stopping') ? $style['blink'] : '']"
        @click="
          if (showState.playbackCursor != null) {
            api.sendStop(showState.playbackCursor);
          }
        "
      />
      <v-btn
        :icon="mdiPlay"
        :active="isCueStatus('playing') || isCueStatus('preWaiting')"
        :disabled="showState.playbackCursor == null"
        active-color="green"
        class="flex-grow-1 border-background"
        :class="[isCueStatus('preWaiting') ? $style['blink'] : '']"
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
      <v-btn
        :icon="mdiPause"
        :active="isCueStatus('paused') || isCueStatus('loaded')"
        :disabled="showState.playbackCursor == null"
        active-color="orange"
        class="flex-grow-1 border-background"
        :class="[isCueStatus('loaded') ? $style['blink'] : '']"
        @click="handleReadyPauseButton"
      />
    </v-btn-group>
  </v-sheet>
</template>
<script setup lang="ts">
import { computed } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import { storeToRefs } from 'pinia';
import { useApi } from '../../api';
import { useShowState } from '../../stores/showstate';
import { buildCueName, firstUpper, getCueIcon, secondsToFormat } from '../../utils';
import { PlaybackStatus } from '../../types/PlaybackStatus';
import { mdiArrowDownRight, mdiArrowUpLeft, mdiFastForward, mdiPause, mdiPlay, mdiRepeat, mdiRewind, mdiSkipNext, mdiSkipPrevious, mdiStop } from '@mdi/js';
import SeekBar from './SeekBar.vue';
import { useUiSettings } from '../../stores/uiSettings';
import { useAssetResult } from '../../stores/assetResult';

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
    if (playbackCursorCue.value.number.trim() != '') {
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
      return (status: PlaybackStatus) => activeCue.status == status;
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
    let cursorIndex = showModel.flatCueList.findIndex(item => item.cue.id == showState.playbackCursor);
    const currentLevel = showModel.flatCueList[cursorIndex]!.level;

    cursorIndex--;
    let cursorCueRef = showModel.flatCueList[cursorIndex];
    if (cursorCueRef == null) return;

    while (cursorCueRef.level != currentLevel) {
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
    let cursorIndex = showModel.flatCueList.findIndex(item => item.cue.id == showState.playbackCursor);
    const currentLevel = showModel.flatCueList[cursorIndex]!.level;

    cursorIndex++;
    let cursorCueRef = showModel.flatCueList[cursorIndex];
    if (cursorCueRef == null) return;

    while (cursorCueRef.level != currentLevel) {
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
  let cursorEntry = showModel.flatCueList.find(item => item.cue.id == showState.playbackCursor);
  if (cursorEntry == null || cursorEntry.parent == null) return;

  api.setPlaybackCursor(cursorEntry.parent);
};

const skipToChild = () => {
  let cursorEntry = showModel.flatCueList.find(item => item.cue.id == showState.playbackCursor);
  if (cursorEntry == null || cursorEntry.cue.params.type != 'group') return;
  const firstChild = cursorEntry.cue.params.children[0];
  if (firstChild != null) {
    api.setPlaybackCursor(firstChild.id);
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

<style lang="css" module>
  .blink {
    animation: flash 1s ease infinite;
  }
  @keyframes flash {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.25;
    }
  }
</style>
