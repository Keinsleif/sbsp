<template>
  <v-sheet flat class="d-flex flex-column pa-4 ga-2">
    <time-range
      v-model="range"
      :disabled="selectedCue!.id in showState.activeCues"
      :duration="assetResult.duration[selectedCue!.id]"
      @update="saveEditorValue('range')"
    ></time-range>
    <waveform-viewer
      v-model="uiState.selected"
      :volume="volume"
      :start-time="range[0]"
      :end-time="range[1]"
      :duration="assetResult.duration[selectedCue!.id]"
    ></waveform-viewer>
    <div class="d-flex flex-row ga-4">
      <volume-fader class="mt-4" v-model="volume" label="Volume" @update:model-value="saveEditorValue('volume')" />
      <v-divider vertical inset thickness="2" />
      <panning-fader class="mt-4" label="Pan" @update:model-value="saveEditorValue('pan')" />
    </div>
  </v-sheet>
</template>

<script setup lang="ts">
import { computed, ref, toRaw, watch } from 'vue';
import { useUiState } from '../../stores/uistate';
import { useShowModel } from '../../stores/showmodel';
import { invoke } from '@tauri-apps/api/core';
import VolumeFader from '../input/VolumeFader.vue';
import PanningFader from '../input/PanningFader.vue';
import WaveformViewer from './WaveformViewer.vue';
import { throttle } from 'vuetify/lib/util/helpers.mjs';
import TimeRange from '../input/TimeRange.vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showstate';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
const assetResult = useAssetResult();

const selectedCue = computed(() => {
  return uiState.selected != null ? showModel.cues.find((cue) => cue.id === uiState.selected) : null;
});

const range = ref([
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.startTime : 0,
  selectedCue.value != null && selectedCue.value.params.type == 'audio'
    ? selectedCue.value.params.endTime
    : assetResult.duration[selectedCue.value!.id],
] as [number | null, number | null]);

const volume = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.volume : 0,
);

const panning = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.pan : 0,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'audio') {
    return;
  }
  range.value = [selectedCue.value.params.startTime, selectedCue.value.params.endTime] as [
    number | null,
    number | null,
  ];
  volume.value = selectedCue.value.params.volume;
  panning.value = selectedCue.value.params.pan;
});

const saveEditorValue = throttle((name: string) => {
  if (selectedCue.value == null) {
    return;
  }
  const newCue = structuredClone(toRaw(selectedCue.value));
  if (newCue.params.type != 'audio') {
    return;
  }
  switch (name) {
    case 'range':
      newCue.params.startTime = range.value[0];
      newCue.params.endTime = range.value[1];
      break;
    case 'volume':
      newCue.params.volume = volume.value;
      break;
    case 'pan':
      newCue.params.pan = panning.value;
      break;
  }
  invoke('update_cue', { cue: newCue });
}, 500);
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
