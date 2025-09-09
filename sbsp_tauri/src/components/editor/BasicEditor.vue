<template>
  <v-sheet v-if="selectedCue != null" flat class="d-flex flex-row pa-4 ga-4">
    <v-sheet flat class="d-flex flex-column ga-2" width="175px">
      <text-input v-model="number" label="Number" @update="saveEditorValue('number')"></text-input>
      <time-input
        v-model="duration"
        :disabled="selectedCue.params.type == 'audio'"
        label="Duration"
        @update="saveEditorValue('duration')"
      ></time-input>
      <time-input v-model="preWait" label="Pre-Wait" @update="saveEditorValue('preWait')"></time-input>
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
        @update:modelValue="saveEditorValue('sequence')"
      ></v-select>
      <time-input
        v-model="postWait"
        :disabled="sequence != 'autoContinue'"
        label="Post-Wait"
        @update="saveEditorValue('postWait')"
      ></time-input>
    </v-sheet>
    <v-sheet flat class="d-flex flex-grow-1 flex-column ga-2 justify-start">
      <text-input
        :placeholder="buildCueName(selectedCue)"
        v-model="name"
        label="Name"
        alignInput="left"
        class="flex-grow-0"
        @update="saveEditorValue('name')"
      ></text-input>
      <text-input v-model="notes" label="Notes" type="area" @update="saveEditorValue('notes')"></text-input>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { computed, ref, toRaw, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useShowModel } from '../../stores/showmodel';
import { buildCueName, calculateDuration } from '../../utils';
import { useAssetResult } from '../../stores/assetResult';
import TextInput from '../input/TextInput.vue';
import TimeInput from '../input/TimeInput.vue';

const showModel = useShowModel();
const assetResult = useAssetResult();

const props = withDefaults(
  defineProps<{
    selectedId: string | null;
  }>(),
  {
    selectedId: null,
  },
);

const selectedCue = computed(() => {
  return props.selectedId != null ? showModel.cues.find((cue) => cue.id === props.selectedId) : null;
});

const getDuration = (): number | null => {
  if (selectedCue.value == null) {
    return null;
  }
  switch (selectedCue.value.params.type) {
    case 'wait':
      return selectedCue.value.params.duration;
    case 'audio':
      return calculateDuration(selectedCue.value.params, assetResult.results[selectedCue.value.id].duration);
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

const saveEditorValue = (paramName: string) => {
  if (selectedCue.value == null) {
    return;
  }
  const newCue = structuredClone(toRaw(selectedCue.value));
  switch (paramName) {
    case 'number':
      if (number.value != null) {
        newCue.number = number.value;
      }
      break;
    case 'duration':
    case 'preWait':
      if (preWait.value != null) {
        newCue.preWait = preWait.value;
      }
      break;
    case 'sequence':
      if (sequence.value != null) {
        newCue.sequence.type = sequence.value;
        if (newCue.sequence.type == 'autoContinue') {
          newCue.sequence.postWait = 0;
        }
        document.body.focus();
      }
      break;
    case 'postWait':
      if (postWait.value != null && newCue.sequence.type == 'autoContinue') {
        newCue.sequence.postWait = postWait.value;
      }
      break;
    case 'name': {
      if (name.value != null) {
        const newName = name.value.trim();
        if (newName == '') {
          newCue.name = null;
        } else {
          newCue.name = newName;
        }
      }
      break;
    }
    case 'notes':
      if (notes.value != null) {
        newCue.notes = notes.value;
      }
      break;
  }
  invoke('update_cue', { cue: newCue });
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
