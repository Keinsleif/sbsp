<template>
  <v-sheet class="d-flex flex-column ma-0 w-100 ga-4 pl-4 pr-4">
    <v-sheet class="d-flex flex-row ma-0 w-100 ga-4">
      <div class="d-flex align-center border">
        <div class="d-flex align-end pl-3 pr-3 text-center text-h2">
          <span>{{ String(time.getHours()).padStart(2, '0') }}</span
          >:<span>{{ String(time.getMinutes()).padStart(2, '0') }}</span
          >.<span class="text-h3">{{ String(time.getSeconds()).padStart(2, '0') }}</span>
        </div>
      </div>
      <div class="d-flex flex-column ma-0 flex-grow-1">
        <v-sheet class="pa-2 rounded mb-1 border-md" height="42px">
          {{ playbackCursorCueTitle }}
        </v-sheet>
        <v-sheet class="pa-2 pb-0 rounded border-md text-pre-wrap overflow-auto" height="64px">{{
          playbackCursorCue != null ? playbackCursorCue.notes : ''
        }}</v-sheet>
      </div>
    </v-sheet>
    <v-sheet class="d-flex flex-columns ga-4">
      <v-btn-group variant="flat" divided border class="ml-0 mr-auto">
        <v-btn
          :icon="mdiStop"
          active-color="error"
          :disabled="showState.playbackCursor == null"
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
          active-color="success"
          :class="[isCueStatus('PreWaiting') ? $style['blink'] : '']"
          @click="invoke('go').catch((e) => console.log(e.toString()))"
        ></v-btn>
        <v-btn
          :icon="mdiPause"
          :active="isCueStatus('Paused') || isCueStatus('Loaded')"
          :disabled="showState.playbackCursor == null"
          active-color="warning"
          :class="[isCueStatus('Loaded') ? $style['blink'] : '']"
          @click="handleReadyPauseButton"
        ></v-btn>
      </v-btn-group>
      <v-btn-group variant="tonal" divided>
        <v-btn :icon="mdiVolumeHigh" @click="addEmptyCue('audio')"></v-btn>
        <v-btn :icon="mdiTimerSandEmpty" @click="addEmptyCue('wait')"></v-btn>
      </v-btn-group>
      <v-btn-group variant="tonal" divided>
        <v-btn :icon="mdiPlayCircleOutline"></v-btn>
        <v-btn :icon="mdiStopCircleOutline"></v-btn>
        <v-btn :icon="mdiPauseCircleOutline"></v-btn>
        <v-btn :icon="mdiCheckCircleOutline"></v-btn>
      </v-btn-group>
      <v-btn-group variant="tonal" divided>
        <v-btn :icon="isFullscreen ? mdiFullscreenExit : mdiFullscreen" @click="toggleFullscreen"></v-btn>
      </v-btn-group>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import {
  mdiCheckCircleOutline,
  mdiFullscreen,
  mdiFullscreenExit,
  mdiPause,
  mdiPauseCircleOutline,
  mdiPlay,
  mdiPlayCircleOutline,
  mdiStop,
  mdiStopCircleOutline,
  mdiTimerSandEmpty,
  mdiVolumeHigh,
} from '@mdi/js';
import { useShowModel } from '../stores/showmodel';
import { computed, onMounted, onUnmounted, ref, toRaw } from 'vue';
import { useShowState } from '../stores/showstate';
import { PlaybackStatus } from '../types/PlaybackStatus';
import { invoke } from '@tauri-apps/api/core';
import { v4 } from 'uuid';
import { useUiState } from '../stores/uistate';
import { buildCueName } from '../utils';
import type { Cue } from '../types/Cue';
import { open } from '@tauri-apps/plugin-dialog';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();

const isFullscreen = ref(false);
const updateFullscreenState = () => (isFullscreen.value = !!document.fullscreenElement);
const toggleFullscreen = () => {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen();
  } else {
    document.exitFullscreen();
  }
};

const playbackCursorCue = computed(() => {
  return showState.playbackCursor != null ? showModel.cues.find((cue) => cue.id == showState.playbackCursor) : null;
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

const addEmptyCue = (type: 'audio' | 'wait') => {
  let insertIndex;
  if (uiState.selected) {
    insertIndex = showModel.cues.findIndex((cue) => cue.id == uiState.selected) + 1;
  } else {
    insertIndex = showModel.cues.length;
  }
  if (type == 'audio') {
    open({
      multiple: true,
      filters: [
        {
          name: 'Audio',
          extensions: [
            'aiff',
            'aif',
            'caf',
            'mp4',
            'm4a',
            'mkv',
            'mka',
            'webm',
            'ogg',
            'oga',
            'wav',
            'aac',
            'alac',
            'flac',
            'mp3',
          ],
        },
      ],
    }).then((value) => {
      if (value == null) {
        return;
      }
      if (value.length === 1) {
        let newCue = structuredClone(toRaw(showModel.settings.template['audio']));
        if (newCue.params.type != 'audio') return;
        newCue.id = v4();
        newCue.params.target = value[0];
        invoke('add_cue', { cue: newCue, atIndex: insertIndex }).catch((e) => console.log(e.toString()));
      } else {
        const newCues: Cue[] = [];
        value.forEach((filename) => {
          let newCue = structuredClone(toRaw(showModel.settings.template['audio']));
          if (newCue.params.type != 'audio') return;
          newCue.id = v4();
          newCue.params.target = filename;
          newCues.push(newCue);
        });
        invoke('add_cues', { cues: newCues, atIndex: insertIndex }).catch((e) => console.log(e.toString()));
      }
    });
  } else if (type == 'wait') {
    const newCue = structuredClone(toRaw(showModel.settings.template['wait']));
    newCue.id = v4();
    invoke('add_cue', { cue: newCue, atIndex: insertIndex }).catch((e) => console.log(e.toString()));
  }
};

const handleReadyPauseButton = () => {
  if (showState.playbackCursor != null) {
    switch (showState.activeCues[showState.playbackCursor]?.status) {
      case 'Playing': {
        invoke('pause', { cueId: showState.playbackCursor }).catch((e) => console.error(e));
        break;
      }
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

const time = ref(new Date());
const ticker = ref();

onMounted(() => {
  ticker.value = setInterval(() => {
    time.value = new Date();
  }, 100);
  document.addEventListener('fullscreenchange', updateFullscreenState);
});

onUnmounted(() => {
  clearInterval(ticker.value);
  document.removeEventListener('fullscreenchange', updateFullscreenState);
});
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
