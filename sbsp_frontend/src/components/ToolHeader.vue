<template>
  <div class="d-flex flex-row ma-0 w-100 h-100 ga-4 pa-4">
    <div class="d-flex flex-column ga-4 flex-grow-0 align-start">
      <div class="d-flex align-center border flex-grow-1" :class="hasFocus ? '' : 'bg-red'">
        <div class="d-flex align-end pl-3 pr-3 text-center" style="font-size: 4em; line-height: 1">
          <span>{{ String(time.getHours()).padStart(2, '0') }}</span
          >:<span>{{ String(time.getMinutes()).padStart(2, '0') }}</span
          >.<span style="font-size: 32pt; line-height: 1">{{ String(time.getSeconds()).padStart(2, '0') }}</span>
        </div>
      </div>
      <div class="d-flex flex-column ga-4">
        <v-btn-group variant="flat" divided border>
          <v-btn
            :icon="mdiStop"
            :active="isCueStatus('Stopping')"
            active-color="red"
            :disabled="showState.playbackCursor == null"
            :class="[isCueStatus('Stopping') ? $style['blink'] : '']"
            @click="
              if (showState.playbackCursor != null) {
                invoke('stop', { cueId: showState.playbackCursor }).catch((e) => console.log(e.toString()));
              }
            "
          ></v-btn>
          <v-btn
            :icon="mdiPlay"
            :active="isCueStatus('Playing') || isCueStatus('PreWaiting')"
            :disabled="showState.playbackCursor == null"
            active-color="green"
            :class="[isCueStatus('PreWaiting') ? $style['blink'] : '']"
            @click="
              if (isCueStatus('Paused') || isCueStatus('PreWaitPaused')) {
                invoke('resume', { cueId: showState.playbackCursor }).catch((e) => console.error(e));
              } else {
                invoke('go').catch((e) => console.log(e.toString()));
              }
            "
          ></v-btn>
          <v-btn
            :icon="mdiPause"
            :active="isCueStatus('Paused') || isCueStatus('Loaded')"
            :disabled="showState.playbackCursor == null"
            active-color="orange"
            :class="[isCueStatus('Loaded') ? $style['blink'] : '']"
            @click="handleReadyPauseButton"
          ></v-btn>
        </v-btn-group>
      </div>
    </div>
    <div class="d-flex flex-column ga-4 flex-grow-1">
      <div class="d-flex flex-column ma-0 flex-grow-1">
        <v-sheet class="pa-2 rounded mb-1 border-md" height="42px">
          {{ playbackCursorCueTitle }}
        </v-sheet>
        <v-sheet class="pa-2 pb-0 rounded border-md text-pre-wrap overflow-auto flex-grow-1" height="1px">{{
          playbackCursorCue != null ? playbackCursorCue.notes : ''
        }}</v-sheet>
      </div>
      <div :class="uiState.mode == 'edit' ? 'd-flex' : 'd-none'" class="flex-row ga-4 justify-end">
        <v-btn-group variant="tonal" divided>
          <v-btn :icon="mdiVolumeHigh" @click="showModel.addEmptyAudioCue()"></v-btn>
          <v-btn :icon="mdiTimerSandEmpty" @click="showModel.addEmptyWaitCue()"></v-btn>
          <v-btn :icon="mdiChartBellCurveCumulative" @click="showModel.addEmptyFadeCue()"></v-btn>
        </v-btn-group>
        <v-btn-group variant="tonal" divided>
          <v-btn :icon="mdiGroup" @click="showModel.addEmptyGroupCue()"></v-btn>
        </v-btn-group>
        <v-btn-group variant="tonal" divided>
          <v-btn :icon="mdiPlayCircleOutline" @click="showModel.addEmptyPlaybackCue('start')"></v-btn>
          <v-btn :icon="mdiStopCircleOutline" @click="showModel.addEmptyPlaybackCue('stop')"></v-btn>
          <v-btn :icon="mdiPauseCircleOutline" @click="showModel.addEmptyPlaybackCue('pause')"></v-btn>
          <v-btn :icon="mdiUploadCircleOutline" @click="showModel.addEmptyPlaybackCue('load')"></v-btn>
        </v-btn-group>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
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
import { useShowModel } from '../stores/showmodel';
import { computed } from 'vue';
import { useShowState } from '../stores/showstate';
import { PlaybackStatus } from '../types/PlaybackStatus';
import { invoke } from '@tauri-apps/api/core';
import { buildCueName } from '../utils';
import { useNow, useWindowFocus } from '@vueuse/core';
import { useUiState } from '../stores/uistate';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();

const hasFocus = useWindowFocus();

const playbackCursorCue = computed(() => {
  return showState.playbackCursor != null ? showModel.getCueById(showState.playbackCursor) : null;
});

const playbackCursorCueTitle = computed(() => {
  return playbackCursorCue.value != null
    ? playbackCursorCue.value.number +
        '・' +
        (playbackCursorCue.value.name != null ? playbackCursorCue.value.name : buildCueName(playbackCursorCue.value))
    : '';
});

const isCueStatus = (status: PlaybackStatus) => {
  if (showState.playbackCursor != null) {
    const activeCue = showState.activeCues[showState.playbackCursor];
    if (activeCue != null) {
      return activeCue.status == status;
    }
  }
  return false;
};

const handleReadyPauseButton = () => {
  if (showState.playbackCursor != null) {
    switch (showState.activeCues[showState.playbackCursor]?.status) {
      case 'PreWaiting':
      case 'Playing': {
        invoke('pause', { cueId: showState.playbackCursor }).catch((e) => console.error(e));
        break;
      }
      case 'PreWaitPaused':
      case 'Paused': {
        invoke('resume', { cueId: showState.playbackCursor }).catch((e) => console.error(e));
        break;
      }
      case undefined: {
        invoke('load', { cueId: showState.playbackCursor }).catch((e) => console.error(e));
        break;
      }
    }
  }
};

const time = useNow();
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
    opacity: 0;
  }
}
</style>
