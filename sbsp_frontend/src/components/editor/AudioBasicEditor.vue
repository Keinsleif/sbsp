<template>
  <v-sheet flat class="d-flex flex-column pa-4">
    <v-text-field
      hide-details
      persistent-placeholder
      v-model="target"
      :label="t('main.bottomEditor.audio.targetFile')"
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
        :label="t('main.bottomEditor.audio.loadEntireFileOnMemory')"
        @update:model-value="saveEditorValue"
      >
        <v-tooltip activator="parent" location="end">{{ t('general.forExpertWarning') }}</v-tooltip>
      </v-checkbox>
    </v-sheet>
    <v-sheet flat class="d-flex ga-4 justify-space-evenly" :class="mdAndDown ? 'flex-column' : 'flex-row'">
      <fade-param-input
        v-model="fadeInParam"
        :label="t('main.bottomEditor.audio.fadeIn')"
        condition="in"
        :disabled="selectedCue!.id in showState.activeCues"
        @update="saveEditorValue"
      ></fade-param-input>
      <fade-param-input
        v-model="fadeOutParam"
        :label="t('main.bottomEditor.audio.fadeOut')"
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
import { ref, watch } from 'vue';
import FadeParamInput from '../input/FadeParamInput.vue';
import { useShowState } from '../../stores/showstate';
import type { Cue } from '../../types/Cue';
import { useUiState } from '../../stores/uistate';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';

const { t } = useI18n();
const { mdAndDown } = useDisplay();

const showState = useShowState();
const uiState = useUiState();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

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

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  if (selectedCue.value.params.type != 'audio') {
    return;
  }
  selectedCue.value.params.target = target.value;
  selectedCue.value.params.soundType = soundType.value ? 'static' : 'streaming';
  selectedCue.value.params.fadeInParam = fadeInParam.value;
  selectedCue.value.params.fadeOutParam = fadeOutParam.value;
  emit('update');
};

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
  document.body.focus();
  if (uiState.side) {
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
      saveEditorValue();
    });
  }
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
