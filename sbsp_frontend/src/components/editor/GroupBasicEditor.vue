<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, watch } from 'vue';
import type { Cue } from '../../types/Cue';
import { useI18n } from 'vue-i18n';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import { useShowState } from '@/stores/showState.ts';

const { t } = useI18n();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const showState = useShowState();

const mode = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'group'
    ? selectedCue.value.params.mode.type
    : undefined,
);

const repeat = ref(
  selectedCue.value != null &&
    selectedCue.value.params.type === 'group' &&
    selectedCue.value.params.mode.type === 'playlist'
    ? selectedCue.value.params.mode.repeat
    : undefined,
);

const enter = ref(
  selectedCue.value != null &&
    selectedCue.value.params.type === 'group' &&
    selectedCue.value.params.mode.type === 'startFirst'
    ? selectedCue.value.params.mode.enter
    : undefined,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type !== 'group') {
    return;
  }
  mode.value = selectedCue.value.params.mode.type;
  repeat.value =
    selectedCue.value.params.mode.type === 'playlist'
      ? selectedCue.value.params.mode.repeat
      : undefined;
  enter.value =
    selectedCue.value.params.mode.type === 'startFirst'
      ? selectedCue.value.params.mode.enter
      : undefined;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || selectedCue.value.params.type !== 'group') {
    return;
  }
  if (mode.value != null) {
    if (mode.value !== selectedCue.value.params.mode.type) {
      selectedCue.value.params.mode.type = mode.value;
      if (selectedCue.value.params.mode.type === 'playlist') {
        selectedCue.value.params.mode.repeat = true;
      } else if (selectedCue.value.params.mode.type === 'startFirst') {
        selectedCue.value.params.mode.enter = false;
      }
    }
    if (
      selectedCue.value.params.mode.type === 'playlist' &&
      repeat.value != null &&
      repeat.value !== selectedCue.value.params.mode.repeat
    ) {
      selectedCue.value.params.mode.repeat = repeat.value;
    }
    if (
      selectedCue.value.params.mode.type === 'startFirst' &&
      enter.value != null &&
      enter.value !== selectedCue.value.params.mode.enter
    ) {
      selectedCue.value.params.mode.enter = enter.value;
    }
  }
  emit('update');
};

const isActive = computed(() => {
  return selectedCue.value != null && selectedCue.value.id in showState.activeCues;
})
</script>

<template>
  <div class="flex flex-col gap-4 p-4">
    <select-wrapper
      v-model="mode"
      :label="t('main.bottomEditor.group.mode.label')"
      :items="[
        { value: 'playlist', name: t('main.bottomEditor.group.mode.playlist') },
        { value: 'concurrency', name: t('main.bottomEditor.group.mode.concurrency') },
        { value: 'startFirst', name: t('main.bottomEditor.group.mode.startFirst') },
      ]"
      :disabled="isActive"
      autocomplete="off"
      @update:model-value="saveEditorValue"
      @keydown.stop
    />
    <checkbox-wrapper
      v-show="selectedCue != null && mode == 'playlist'"
      v-model="repeat"
      :label="t('main.bottomEditor.timeLevels.repeat')"
      :disabled="isActive"
      @update:model-value="saveEditorValue"
    />
    <checkbox-wrapper
      v-show="selectedCue != null && mode == 'startFirst'"
      v-model="enter"
      :label="t('main.bottomEditor.group.advanceCursorInto')"
      :disabled="isActive"
      @update:model-value="saveEditorValue"
    />
  </div>
</template>
