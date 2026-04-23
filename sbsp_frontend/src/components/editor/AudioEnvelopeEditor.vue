<template>
  <v-sheet class="d-flex flex-column pa-3 ga-3">
    <waveform-envelope-editor
      v-model="selectedCue"
      :height-px="140"
      :start-time="range[0]"
      :end-time="range[1]"
      @update="saveEditorValue"
    />
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import WaveformEnvelopeEditor from './WaveformEnvelopeEditor.vue';
import type { Cue } from '../../types/Cue';
import { useAssetResult } from '../../stores/assetResult';

const assetResult = useAssetResult();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'audio') {
    return;
  }
  range.value = [selectedCue.value.params.startTime, selectedCue.value.params.endTime] as [
      number | null,
      number | null,
  ];
});

const range = ref([
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.startTime : 0,
  selectedCue.value != null && selectedCue.value.params.type == 'audio'
    ? selectedCue.value.params.endTime
    : assetResult.getMetadata(selectedCue.value?.id)?.duration,
] as [number | null, number | null]);

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  if (selectedCue.value.params.type != 'audio') {
    return;
  }
  emit('update');
};
</script>
