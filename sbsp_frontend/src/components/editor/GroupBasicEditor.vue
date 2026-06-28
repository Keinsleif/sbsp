<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import type { Cue } from '../../types/Cue';
import { useI18n } from 'vue-i18n';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';

const { t } = useI18n();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const mode = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'group'
    ? selectedCue.value.params.mode.type
    : null,
);

const repeat = ref(
  selectedCue.value != null &&
    selectedCue.value.params.type === 'group' &&
    selectedCue.value.params.mode.type === 'playlist'
    ? selectedCue.value.params.mode.repeat
    : null,
);

const enter = ref(
  selectedCue.value != null &&
    selectedCue.value.params.type === 'group' &&
    selectedCue.value.params.mode.type === 'startFirst'
    ? selectedCue.value.params.mode.enter
    : null,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type !== 'group') {
    return;
  }
  mode.value = selectedCue.value.params.mode.type;
  repeat.value =
    selectedCue.value.params.mode.type === 'playlist' ? selectedCue.value.params.mode.repeat : null;
  enter.value =
    selectedCue.value.params.mode.type === 'startFirst'
      ? selectedCue.value.params.mode.enter
      : null;
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
      autocomplete="off"
      @update:model-value="saveEditorValue"
      @keydown.stop
    />
    <checkbox-wrapper
      v-show="selectedCue != null && mode == 'playlist'"
      v-model="repeat"
      :label="t('main.bottomEditor.timeLevels.repeat')"
      @update:model-value="saveEditorValue"
    />
    <checkbox-wrapper
      v-show="selectedCue != null && mode == 'startFirst'"
      v-model="enter"
      :label="t('main.bottomEditor.group.advanceCursorInto')"
      @update:model-value="saveEditorValue"
    />
  </div>
</template>
