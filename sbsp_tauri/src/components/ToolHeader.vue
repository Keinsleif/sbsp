<template>
  <v-sheet class="d-flex flex-column ma-0 w-100 ga-4 pl-4 pr-4">
    <v-sheet class="d-flex flex-row ma-0 w-100">
      <div class="d-flex flex-column ma-0 flex-grow-1">
        <v-sheet class="pa-2 rounded mb-1 border-md" height="42px">
          {{selectedCue != null ? selectedCue.number + "・" + selectedCue.name : ""}}
        </v-sheet>
        <v-sheet
          class="pa-2 pb-0 rounded border-md text-pre-wrap overflow-auto"
          height="64px"
        >{{selectedCue != null ? selectedCue.notes : ""}}</v-sheet>
      </div>
    </v-sheet>
    <v-sheet class="d-flex flex-columns ga-4">
      <v-btn-group variant="flat" divided border class="ml-0 mr-auto">
        <v-btn :icon="mdiStop" active-color="error"></v-btn>
        <v-btn :icon="mdiPlay" :active="isCueStatus('Playing')" active-color="success" @click="invoke('go')"></v-btn>
        <v-btn :icon="mdiPause" :active="isCueStatus('Paused')" active-color="warning" :class="[isCueStatus('Paused') ? $style['pause-blink'] : '']"></v-btn>
      </v-btn-group>
      <v-btn-group variant="tonal" divided>
        <v-btn :icon="mdiVolumeHigh"></v-btn>
        <v-btn :icon="mdiTimerSandEmpty"></v-btn>
      </v-btn-group>
      <v-btn-group variant="tonal" divided>
        <v-btn :icon="mdiPlayCircleOutline"></v-btn>
        <v-btn :icon="mdiStopCircleOutline"></v-btn>
        <v-btn :icon="mdiPauseCircleOutline"></v-btn>
        <v-btn :icon="mdiCheckCircleOutline"></v-btn>
      </v-btn-group>
      <v-btn-group variant="tonal" divided>
        <v-btn :icon="mdiFullscreen"></v-btn>
      </v-btn-group>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import {
  mdiCheckCircleOutline,
  mdiFullscreen,
  mdiPause,
  mdiPauseCircleOutline,
  mdiPlay,
  mdiPlayCircleOutline,
  mdiStop,
  mdiStopCircleOutline,
  mdiTimerSandEmpty,
  mdiVolumeHigh,
} from "@mdi/js";
import { useShowModel } from "../stores/showmodel";
import { computed } from "vue";
import { useUiState } from "../stores/uistate";
import { useShowState } from "../stores/showstate";
import { PlaybackStatus } from "../types/state/PlaybackStatus";
import { invoke } from "@tauri-apps/api/core";

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();

const selectedCue = computed(() => {
  return uiState.selected != null ? showModel.cues[uiState.selected] : null;
})

const isCueStatus = computed(() => (status: PlaybackStatus) => {
  if (uiState.selected != null) {
    const activeCue = showState.activeCues[showModel.cues[uiState.selected].id];
    if (activeCue != null) {
      return activeCue.status == status;
    }
  }
  return false;
})
</script>

<style lang="css" module>
.pause-blink {
  animation: flash 1s ease infinite;
}
@keyframes flash {
  0%,100% {
    opacity: 1;
  }
  50% {
    opacity: 0;
  }
}
</style>