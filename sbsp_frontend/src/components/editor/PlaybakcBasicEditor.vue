<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import type { Cue } from '../../types/Cue';
import CueSelect from '../input/CueSelect.vue';
import { NIL } from 'uuid';
import { useI18n } from 'vue-i18n';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';

const { t } = useI18n();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const target = ref(
  selectedCue.value != null &&
    (selectedCue.value.params.type === 'start' ||
      selectedCue.value.params.type === 'stop' ||
      selectedCue.value.params.type === 'pause' ||
      selectedCue.value.params.type === 'load') &&
    selectedCue.value.params.target !== NIL
    ? selectedCue.value.params.target
    : '',
);
const hard = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'stop'
    ? selectedCue.value.params.hard
    : undefined,
);

watch(selectedCue, () => {
  if (
    selectedCue.value == null ||
    !(
      selectedCue.value.params.type === 'start' ||
      selectedCue.value.params.type === 'stop' ||
      selectedCue.value.params.type === 'pause' ||
      selectedCue.value.params.type === 'load'
    )
  ) {
    return;
  }

  target.value = selectedCue.value.params.target;
  hard.value = selectedCue.value.params.type === 'stop' ? selectedCue.value.params.hard : undefined;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || sliderChanging.value === true) {
    return;
  }
  if (
    !(
      selectedCue.value.params.type === 'start' ||
      selectedCue.value.params.type === 'stop' ||
      selectedCue.value.params.type === 'pause' ||
      selectedCue.value.params.type === 'load'
    )
  ) {
    return;
  }
  selectedCue.value.params.target = target.value;
  if (selectedCue.value.params.type === 'stop') {
    selectedCue.value.params.hard = hard.value || false;
  }
  emit('update');
};
</script>

<template>
  <div class="flex flex-col gap-3 p-4">
    <cue-select
      v-model="target"
      class="grow-0"
      :label="t('main.bottomEditor.targetCue')"
      cue-type="all"
      :exclude="selectedCue != null ? selectedCue.id : ''"
      @update="saveEditorValue"
    />
    <checkbox-wrapper
      v-show="selectedCue != null && selectedCue.params.type == 'stop'"
      v-model="hard"
      :label="t('main.bottomEditor.playback.hard')"
      @update:model-value="saveEditorValue"
    />
  </div>
</template>
