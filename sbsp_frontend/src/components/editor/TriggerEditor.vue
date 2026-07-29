<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import type { Cue } from '../../types/Cue';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
// import { useI18n } from 'vue-i18n';

// const { t } = useI18n();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const cursorAdvanceTrigger = ref();
const treatStopAsCompleted = ref();

watch(selectedCue, () => {
  cursorAdvanceTrigger.value = selectedCue.value?.cursorAdvanceTrigger ?? null;
  treatStopAsCompleted.value = selectedCue.value?.treatStopAsCompleted ?? null;
}, { immediate: true });

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  selectedCue.value.cursorAdvanceTrigger = cursorAdvanceTrigger.value;
  selectedCue.value.treatStopAsCompleted = treatStopAsCompleted.value;
  emit('update');
};
</script>

<template>
  <div class="flex min-w-180 flex-col gap-2 p-3">
    <select-wrapper
      v-model="cursorAdvanceTrigger"
      class="grow-0"
      :label="'Playback cursor advance trigger override'"
      :items="[
        { value: 'inherit', name: 'Inherit' },
        { value: 'onTriggered', name: 'On Triggered' },
        { value: 'onCompleted', name: 'On Completed' },
        { value: 'manual', name: 'Manual' },
      ]"
      autocomplete="off"
      @keydown.stop
    />
    <checkbox-wrapper
      v-model="treatStopAsCompleted"
      class="self-start"
      :label="'Treat Stop as Completed'"
      @update:model-value="saveEditorValue"
    />
  </div>
</template>
