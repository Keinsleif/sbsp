<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import type { Cue } from '../../types/Cue';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import { useI18n } from 'vue-i18n';
import type { CueCursorAdvanceTriggerOverride } from '@/types/CueCursorAdvanceTriggerOverride.ts';

const { t } = useI18n();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const cursorAdvanceTriggerOverride = ref<CueCursorAdvanceTriggerOverride>('none');
const treatStopAsCompleted = ref<boolean>(false);

watch(
  selectedCue,
  () => {
    cursorAdvanceTriggerOverride.value = selectedCue.value?.cursorAdvanceTriggerOverride ?? 'none';
    treatStopAsCompleted.value = selectedCue.value?.treatStopAsCompleted ?? false;
  },
  { immediate: true },
);

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  selectedCue.value.cursorAdvanceTriggerOverride = cursorAdvanceTriggerOverride.value;
  selectedCue.value.treatStopAsCompleted = treatStopAsCompleted.value;
  emit('update');
};
</script>

<template>
  <div class="flex min-w-180 flex-col gap-2 p-3">
    <select-wrapper
      v-model="cursorAdvanceTriggerOverride"
      class="grow-0"
      :label="t('main.bottomEditor.trigger.cursorAdvanceTriggerOverride.title')"
      :items="[
        { value: 'none', name: t('main.bottomEditor.trigger.cursorAdvanceTrigger.none') },
        {
          value: 'onTriggered',
          name: t('main.bottomEditor.trigger.cursorAdvanceTrigger.onTriggered'),
        },
        {
          value: 'onCompleted',
          name: t('main.bottomEditor.trigger.cursorAdvanceTrigger.onCompleted'),
        },
        { value: 'manual', name: t('main.bottomEditor.trigger.cursorAdvanceTrigger.manual') },
      ]"
      autocomplete="off"
      @update:model-value="saveEditorValue"
      @keydown.stop
    />
    <checkbox-wrapper
      v-model="treatStopAsCompleted"
      class="self-start"
      :label="t('main.bottomEditor.trigger.treatStopAsCompleted')"
      @update:model-value="saveEditorValue"
    />
  </div>
</template>
