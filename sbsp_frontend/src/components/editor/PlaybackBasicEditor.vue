<template>
  <v-sheet flat class="d-flex flex-column pa-4 ga-2">
    <cue-select
      v-model="target"
      class="flex-grow-0"
      :label="t('main.bottomEditor.targetCue')"
      cueTypes="all"
      :exclude="selectedCue != null ? selectedCue.id : ''"
      @update="saveEditorValue"
    />
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import type { Cue } from '../../types/Cue';
import CueSelect from '../input/CueSelect.vue';
import { NIL } from 'uuid';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const target = ref(
  selectedCue.value != null &&
    (selectedCue.value.params.type == 'start' ||
      selectedCue.value.params.type == 'stop' ||
      selectedCue.value.params.type == 'pause' ||
      selectedCue.value.params.type == 'load') &&
    selectedCue.value.params.target != NIL
    ? selectedCue.value.params.target
    : '',
);

watch(selectedCue, () => {
  if (
    selectedCue.value == null ||
    !(
      selectedCue.value.params.type == 'start' ||
      selectedCue.value.params.type == 'stop' ||
      selectedCue.value.params.type == 'pause' ||
      selectedCue.value.params.type == 'load'
    )
  ) {
    return;
  }

  target.value = selectedCue.value.params.target;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || sliderChanging.value === true) {
    return;
  }
  if (
    !(
      selectedCue.value.params.type == 'start' ||
      selectedCue.value.params.type == 'stop' ||
      selectedCue.value.params.type == 'pause' ||
      selectedCue.value.params.type == 'load'
    )
  ) {
    return;
  }
  selectedCue.value.params.target = target.value;
  emit('update');
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
