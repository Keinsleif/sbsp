<template>
  <v-sheet flat class="d-flex flex-column pa-4">
    <v-text-field
      hide-details
      persistent-placeholder
      v-model="target"
      label="Target"
      variant="outlined"
      density="compact"
      :disabled="selectedCue!.id in showState.activeCues"
      :class="$style['centered-input']"
      @blur="saveEditorValue"
      @keydown.enter="$event.target.blur()"
      @keydown.esc="
        resetEditorValue('target');
        $event.target.blur();
      "
    >
      <template v-slot:append>
        <v-btn
          :active="false"
          density="compact"
          :disabled="selectedCue!.id in showState.activeCues"
          :icon="mdiFileMusic"
          @click="pickFile"
        ></v-btn>
      </template>
    </v-text-field>
    <v-sheet flat class="d-flex flex-row ga-4 mt-4">
      <v-checkbox
        hide-details
        v-model="soundType"
        :disabled="selectedCue!.id in showState.activeCues"
        density="compact"
        label="Load entire file on memory"
        @update:model-value="saveEditorValue"
      >
        <v-tooltip activator="parent" location="end">Change this only if you know what you're doing.</v-tooltip>
      </v-checkbox>
    </v-sheet>
    <v-sheet flat class="d-flex flex-row ga-4 justify-space-evenly">
      <fade-param-input
        v-model="fadeInParam"
        label="Fade In"
        condition="in"
        :disabled="selectedCue!.id in showState.activeCues"
        @update="saveEditorValue"
      ></fade-param-input>
      <fade-param-input
        v-model="fadeOutParam"
        label="Fade Out"
        condition="out"
        :disabled="selectedCue!.id in showState.activeCues"
        @update="saveEditorValue"
      ></fade-param-input>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { mdiFileMusic } from '@mdi/js';
import { open } from '@tauri-apps/plugin-dialog';
import { computed, ref, toRaw, watch } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import { invoke } from '@tauri-apps/api/core';
import FadeParamInput from '../input/FadeParamInput.vue';
import { debounce } from 'vuetify/lib/util/helpers.mjs';
import { useShowState } from '../../stores/showstate';

const showModel = useShowModel();
const showState = useShowState();

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

const target = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.target : '',
);

const soundType = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio'
    ? selectedCue.value.params.soundType == 'static'
    : false,
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
  soundType.value = selectedCue.value.params.soundType == 'static';
  fadeInParam.value = selectedCue.value.params.fadeInParam;
  fadeOutParam.value = selectedCue.value.params.fadeOutParam;
});

const saveEditorValue = debounce(() => {
  if (selectedCue.value == null) {
    return;
  }
  const newCue = structuredClone(toRaw(selectedCue.value));
  if (newCue.params.type != 'audio') {
    return;
  }
  newCue.params.target = target.value;
  newCue.params.soundType = soundType.value ? 'static' : 'streaming';
  newCue.params.fadeInParam = fadeInParam.value;
  newCue.params.fadeOutParam = fadeOutParam.value;
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
  document.body.focus();
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
