<template>
  <v-sheet flat class="d-flex flex-column pa-4">
    <v-text-field
      hide-details
      persistent-placeholder
      :append-icon="mdiFileMusic"
      v-model="target"
      label="Target"
      variant="outlined"
      density="compact"
      :class="$style['centered-input']"
      @blur="saveEditorValue('target')"
      @keydown.enter="$event.target.blur()"
      @keydown.esc="
        resetEditorValue('target');
        $event.target.blur();
      "
      @click:append="pickFile()"
    ></v-text-field>
    <v-sheet flat class="d-flex flex-row ga-4 justify-space-evenly">
      <fade-param-input
        v-model="fadeInParam"
        label="Fade In"
        @update="saveEditorValue('fadeInParam')"
      ></fade-param-input>
      <fade-param-input
        v-model="fadeOutParam"
        label="Fade Out"
        @update="saveEditorValue('fadeOutParam')"
      ></fade-param-input>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { mdiFileMusic } from '@mdi/js';
import { open } from '@tauri-apps/plugin-dialog';
import { computed, ref, toRaw, watch } from 'vue';
import { useUiState } from '../../stores/uistate';
import { useShowModel } from '../../stores/showmodel';
import { invoke } from '@tauri-apps/api/core';
import FadeParamInput from '../input/FadeParamInput.vue';
import { throttle } from 'vuetify/lib/util/helpers.mjs';

const showModel = useShowModel();
const uiState = useUiState();

const selectedCue = computed(() => {
  return uiState.selected != null ? showModel.cues.find((cue) => cue.id === uiState.selected) : null;
});

const target = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.target : '',
);

const fadeInParam = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.fadeInParam : null,
);

const fadeOutParam = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.fadeOutParam : null,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'audio') {
    return;
  }

  target.value = selectedCue.value.params.target;
  fadeInParam.value = selectedCue.value.params.fadeInParam;
  fadeOutParam.value = selectedCue.value.params.fadeOutParam;
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
    case 'target':
      newCue.params.target = target.value;
      break;
    case 'fadeInParam':
      newCue.params.fadeInParam = fadeInParam.value;
      break;
    case 'fadeOutParam':
      newCue.params.fadeOutParam = fadeOutParam.value;
      break;
  }
  invoke('update_cue', { cue: newCue });
}, 500);

const resetEditorValue = (name: string) => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'audio') {
    return;
  }
  switch (name) {
    case 'target':
      target.value = selectedCue.value.params.target;
      break;
  }
};

const pickFile = () => {
  open({
    multiple: false,
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
    target.value = value;
    saveEditorValue('target');
  });
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
