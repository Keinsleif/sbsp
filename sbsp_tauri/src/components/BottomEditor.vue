<template>
  <v-sheet class="overflow-hidden">
    <v-tabs v-model="uiState.editorTab" density="compact" class="border">
      <v-tab border density="compact" value="basics">Basics</v-tab>
      <v-tab border density="compact" value="audio" v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        >Audio</v-tab
      >
      <v-tab border density="compact" value="levels" v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        >Levels</v-tab
      >
    </v-tabs>
    <v-tabs-window v-if="selectedCue != null" v-model="uiState.editorTab">
      <v-tabs-window-item value="basics" reverse-transition="false" transition="false">
        <v-sheet flat class="d-flex flex-row pa-4 ga-4">
          <v-sheet flat class="d-flex flex-column ga-2" width="175px">
            <v-text-field
              hide-details
              persistent-placeholder
              v-model="editorValue.number"
              label="Number"
              variant="outlined"
              density="compact"
              :class="$style['centered-input']"
              autocomplete="off"
              @blur="saveEditorValue"
              @keydown.enter="$event.target.blur()"
              @keydown.esc="
                resetEditorValue();
                $event.target.blur();
              "
            ></v-text-field>
            <v-text-field
              hide-details
              persistent-placeholder
              v-model="editorValue.duration"
              :disabled="selectedCue.params.type == 'audio'"
              label="Duration"
              variant="outlined"
              density="compact"
              :class="$style['centered-input']"
              autocomplete="off"
              @blur="saveEditorValue"
              @keydown.enter="$event.target.blur()"
              @keydown.esc="
                resetEditorValue();
                $event.target.blur();
              "
            ></v-text-field>
            <v-text-field
              hide-details
              persistent-placeholder
              v-model="editorValue.preWait"
              label="Pre-Wait"
              variant="outlined"
              density="compact"
              :class="$style['centered-input']"
              autocomplete="off"
              @blur="saveEditorValue"
              @keydown.enter="$event.target.blur()"
              @keydown.esc="
                resetEditorValue();
                $event.target.blur();
              "
            ></v-text-field>
            <v-select
              hide-details
              persistent-placeholder
              v-model="editorValue.sequence"
              label="ContinueMode"
              :items="[
                { value: 'doNotContinue', name: 'DoNotContinue' },
                { value: 'autoFollow', name: 'Auto-Follow' },
              ]"
              item-value="value"
              item-title="name"
              variant="outlined"
              density="compact"
              autocomplete="off"
              @update:modelValue="saveEditorValue"
            ></v-select>
            <v-text-field
              hide-details
              persistent-placeholder
              v-model="editorValue.postWait"
              :disabled="selectedCue.sequence.type != 'autoFollow'"
              label="Post-Wait"
              variant="outlined"
              density="compact"
              :class="$style['centered-input']"
              autocomplete="off"
              @blur="saveEditorValue"
              @keydown.enter="$event.target.blur()"
              @keydown.esc="
                resetEditorValue();
                $event.target.blur();
              "
            ></v-text-field>
          </v-sheet>
          <v-sheet flat class="d-flex flex-grow-1 flex-column ga-2 justify-start">
            <v-text-field
              hide-details
              persistent-placeholder
              :placeholder="buildCueName(selectedCue)"
              v-model="editorValue.name"
              label="Name"
              variant="outlined"
              density="compact"
              class="flex-grow-0"
              autocomplete="off"
              @blur="saveEditorValue"
              @keydown.enter="$event.target.blur()"
              @keydown.esc="
                resetEditorValue();
                $event.target.blur();
              "
            ></v-text-field>
            <v-textarea
              hide-details
              persistent-placeholder
              no-resize
              v-model="editorValue.notes"
              label="Notes"
              variant="outlined"
              density="compact"
              autocomplete="off"
              @blur="saveEditorValue"
              @keydown.enter="$event.target.blur()"
              @keydown.esc="
                resetEditorValue();
                $event.target.blur();
              "
            ></v-textarea>
          </v-sheet>
        </v-sheet>
      </v-tabs-window-item>
      <v-tabs-window-item
        v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        value="audio"
        reverse-transition="false"
        transition="false"
      >
        <v-sheet flat class="d-flex flex-column pa-4">
          <v-sheet flat class="d-flex flex-column">
            <v-text-field
              hide-details
              persistent-placeholder
              :model-value="selectedCue.params.target"
              label="Target"
              variant="outlined"
              density="compact"
              :class="$style['centered-input']"
            ></v-text-field>
          </v-sheet>
        </v-sheet>
      </v-tabs-window-item>
      <v-tabs-window-item value="levels" reverse-transition="false" transition="false">
        <v-sheet> </v-sheet>
      </v-tabs-window-item>
    </v-tabs-window>
  </v-sheet>
</template>

<script setup lang="ts">
import { useUiState } from '../stores/uistate';
import { useShowModel } from '../stores/showmodel';
import { computed, ref, toRaw, watch } from 'vue';
import { buildCueName, formatToSeconds, secondsToFormat } from '../utils';
import { invoke } from '@tauri-apps/api/core';
import { VTextField } from 'vuetify/components';

const showModel = useShowModel();
const uiState = useUiState();

const selectedCue = computed(() => {
  return uiState.selected != null ? showModel.cues.find((cue) => cue.id === uiState.selected) : null;
});

const computeEditorValue = () => {
  return {
    number: selectedCue.value ? selectedCue.value.number : null,
    name: selectedCue.value ? selectedCue.value.name : null,
    notes: selectedCue.value ? selectedCue.value.notes : null,
    preWait: selectedCue.value ? secondsToFormat(selectedCue.value.preWait) : null,
    duration: selectedCue.value
      ? selectedCue.value.params.type == 'wait'
        ? secondsToFormat(selectedCue.value.params.duration)
        : '00:00.00'
      : null,
    sequence: selectedCue.value ? selectedCue.value.sequence.type : null,
    postWait: selectedCue.value
      ? selectedCue.value.sequence.type != 'doNotContinue'
        ? secondsToFormat(selectedCue.value.sequence.postWait)
        : '00:00.00'
      : null,
  };
};

const editorValue = ref(computeEditorValue());

watch(selectedCue, () => {
  editorValue.value = computeEditorValue();
});

const resetEditorValue = () => {
  editorValue.value = computeEditorValue();
};

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  const newCue = structuredClone(toRaw(selectedCue.value));
  if (editorValue.value.number != null) {
    newCue.number = editorValue.value.number;
  }
  if (editorValue.value.name != null) {
    const newName = editorValue.value.name.trim();
    if (newName == '') {
      newCue.name = null;
    } else {
      newCue.name = newName;
    }
  }
  if (editorValue.value.notes != null) {
    newCue.notes = editorValue.value.notes;
  }
  if (editorValue.value.preWait != null) {
    newCue.preWait = formatToSeconds(editorValue.value.preWait, false);
  }
  if (editorValue.value.duration != null && newCue.params.type == 'wait') {
    newCue.params.duration = formatToSeconds(editorValue.value.duration, false);
  }
  if (editorValue.value.sequence != null) {
    newCue.sequence.type = editorValue.value.sequence;
  }
  if (editorValue.value.postWait != null && newCue.sequence.type != 'doNotContinue') {
    newCue.sequence.postWait = formatToSeconds(editorValue.value.postWait);
  }
  invoke('update_cue', { cue: newCue });
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
