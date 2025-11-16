<template>
  <v-sheet v-if="selectedCue != null" flat class="d-flex flex-row pa-4 ga-4">
    <v-sheet flat class="d-flex flex-column ga-2" width="175px">
      <text-input v-model="number" label="Number" @update="saveEditorValue"></text-input>
      <time-input
        v-model="duration"
        :disabled="selectedCue.params.type == 'audio'"
        label="Duration"
        @update="saveEditorValue"
      ></time-input>
      <time-input v-model="preWait" label="Pre-Wait" @update="saveEditorValue"></time-input>
      <v-select
        hide-details
        persistent-placeholder
        v-model="sequence"
        label="ContinueMode"
        ref="cue_sequence"
        :items="[
          { value: 'doNotContinue', name: 'DoNotContinue' },
          { value: 'autoContinue', name: 'Auto-Continue' },
          { value: 'autoFollow', name: 'Auto-Follow' },
        ]"
        item-value="value"
        item-title="name"
        variant="outlined"
        density="compact"
        autocomplete="off"
        @update:modelValue="saveEditorValue"
        @keydown.stop
      ></v-select>
      <time-input
        v-model="postWait"
        :disabled="sequence != 'autoContinue'"
        label="Post-Wait"
        @update="saveEditorValue"
      ></time-input>
    </v-sheet>
    <v-sheet flat class="d-flex flex-grow-1 flex-column ga-2 justify-start">
      <text-input
        :placeholder="buildCueName(selectedCue)"
        v-model="name"
        label="Name"
        alignInput="left"
        class="flex-grow-0"
        @update="saveEditorValue"
      ></text-input>
      <text-input v-model="notes" label="Notes" type="area" @update="saveEditorValue"></text-input>
      <v-btn
        class="ml-auto mr-0 flex-grow-0"
        density="compact"
        :disabled="!(selectedCue.id in showState.activeCues)"
        @click="insertTimestampToNote"
        >Timestamp</v-btn
      >
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { buildCueName, calculateDuration, secondsToFormat } from '../../utils';
import { useAssetResult } from '../../stores/assetResult';
import TextInput from '../input/TextInput.vue';
import TimeInput from '../input/TimeInput.vue';
import type { Cue } from '../../types/Cue';
import { useShowState } from '../../stores/showstate';

const assetResult = useAssetResult();
const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const getDuration = (): number | null => {
  if (selectedCue.value == null) {
    return null;
  }
  switch (selectedCue.value.params.type) {
    case 'wait':
      return selectedCue.value.params.duration;
    case 'audio':
      return calculateDuration(selectedCue.value.params, assetResult.get(selectedCue.value.id)?.duration);
    case 'fade':
      return selectedCue.value.params.fadeParam.duration;
  }
};

const number = ref(selectedCue.value != null ? selectedCue.value.number : null);
const duration = ref(getDuration());
const preWait = ref(selectedCue.value != null ? selectedCue.value.preWait : null);
const sequence = ref(selectedCue.value != null ? selectedCue.value.sequence.type : null);
const postWait = ref(
  selectedCue.value != null && selectedCue.value.sequence.type != 'doNotContinue'
    ? selectedCue.value.sequence.type == 'autoContinue'
      ? selectedCue.value.sequence.postWait
      : getDuration()
    : null,
);
const name = ref(selectedCue.value != null ? selectedCue.value.name : null);
const notes = ref(selectedCue.value != null ? selectedCue.value.notes : null);

watch(selectedCue, () => {
  number.value = selectedCue.value != null ? selectedCue.value.number : null;
  duration.value = getDuration();
  preWait.value = selectedCue.value != null ? selectedCue.value.preWait : null;
  sequence.value = selectedCue.value != null ? selectedCue.value.sequence.type : null;
  postWait.value =
    selectedCue.value != null && selectedCue.value.sequence.type != 'doNotContinue'
      ? selectedCue.value.sequence.type == 'autoContinue'
        ? selectedCue.value.sequence.postWait
        : getDuration()
      : null;
  name.value = selectedCue.value != null ? selectedCue.value.name : null;
  notes.value = selectedCue.value != null ? selectedCue.value.notes : null;
});

watch(getDuration, () => {
  duration.value = getDuration();
});

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  if (number.value != null) {
    selectedCue.value.number = number.value;
  }
  if (preWait.value != null) {
    selectedCue.value.preWait = preWait.value;
  }
  if (sequence.value != null) {
    selectedCue.value.sequence.type = sequence.value;
    if (selectedCue.value.sequence.type == 'autoContinue') {
      selectedCue.value.sequence.postWait = 0;
    }
  }
  if (postWait.value != null && selectedCue.value.sequence.type == 'autoContinue') {
    selectedCue.value.sequence.postWait = postWait.value;
  }
  if (name.value != null) {
    const newName = name.value.trim();
    if (newName == '') {
      selectedCue.value.name = null;
    } else {
      selectedCue.value.name = newName;
    }
  }
  if (notes.value != null) {
    selectedCue.value.notes = notes.value;
  }
  emit('update');
};

const insertTimestampToNote = () => {
  if (selectedCue.value == null || !(selectedCue.value.id in showState.activeCues)) {
    return;
  }
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue == null) {
    return;
  }
  if (notes.value != null && (notes.value.endsWith('\n') || notes.value == '')) {
    notes.value += `[${secondsToFormat(activeCue.position)}] `;
  } else {
    notes.value += `\n[${secondsToFormat(activeCue.position)}] `;
  }
  saveEditorValue();
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
