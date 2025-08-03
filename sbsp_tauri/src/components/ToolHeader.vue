<template>
  <v-sheet class="d-flex flex-column ma-0 w-100 ga-4 pl-4 pr-4">
    <v-sheet class="d-flex flex-row ma-0 w-100">
      <div class="d-flex flex-column ma-0 flex-grow-1">
        <div class="pa-2 rounded mb-1 border-sm border-current">
          2・Intro Music (Playing)
        </div>
        <v-textarea
          flat
          no-resize
          hide-details
          rows="2"
          density="compact"
          variant="outlined"
          placeholder="Notes"
          base-color="current"
        ></v-textarea>
      </div>
    </v-sheet>
    <v-sheet class="d-flex flex-columns ga-4">
      <v-btn-group variant="tonal" divided class="ml-0 mr-auto">
        <v-btn :icon="mdiStop"></v-btn>
        <v-btn :icon="mdiPlay"></v-btn>
        <v-btn :icon="mdiPause"></v-btn>
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
</script>
