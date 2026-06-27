<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, watch } from 'vue';
import { buildCueName, getDuration, secondsToFormat } from '../../utils';
import type { Cue } from '../../types/Cue';
import { useShowState } from '../../stores/showState';
import { useI18n } from 'vue-i18n';
import { NIL } from 'uuid';
import type { CueChain } from '../../types/CueChain';
import { mdiCircle } from '@mdi/js';
import TextInput from '../input/TextInput.vue';
import TimeInput from '../input/TimeInput.vue';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import CueSelect from '../input/CueSelect.vue';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import TextareaWrapper from '../wrapper/TextareaWrapper.vue';

const { t } = useI18n();

const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const props = withDefaults(
  defineProps<{
    chainOverride?: CueChain | null;
  }>(),
  {
    chainOverride: null,
  },
);
const emit = defineEmits(['update']);

const overridedChain = computed(() =>
  props.chainOverride != null
    ? props.chainOverride
    : selectedCue.value != null
      ? selectedCue.value.chain
      : null,
);

const number = ref(selectedCue.value != null ? selectedCue.value.number : null);
const duration = ref(getDuration(selectedCue.value));
const preWait = ref(selectedCue.value != null ? selectedCue.value.preWait : null);
const chain = ref(overridedChain.value != null ? overridedChain.value.type : null);

const name = ref(selectedCue.value != null ? selectedCue.value.name : null);
const notes = ref(selectedCue.value != null ? selectedCue.value.notes : null);
const color = ref(selectedCue.value != null ? selectedCue.value.color : null);
const target = ref(
  overridedChain.value != null &&
    overridedChain.value.type !== 'doNotChain' &&
    overridedChain.value.targetId !== NIL
    ? overridedChain.value.targetId
    : null,
);

watch(selectedCue, () => {
  number.value = selectedCue.value != null ? selectedCue.value.number : null;
  duration.value = getDuration(selectedCue.value);
  preWait.value = selectedCue.value != null ? selectedCue.value.preWait : null;
  chain.value = overridedChain.value != null ? overridedChain.value.type : null;
  name.value = selectedCue.value != null ? selectedCue.value.name : null;
  notes.value = selectedCue.value != null ? selectedCue.value.notes : null;
  color.value = selectedCue.value != null ? selectedCue.value.color : null;
  target.value =
    overridedChain.value != null &&
    overridedChain.value.type !== 'doNotChain' &&
    overridedChain.value.targetId !== NIL
      ? overridedChain.value.targetId
      : null;
});

watch(
  () => getDuration(selectedCue.value),
  () => {
    const cueDuration = getDuration(selectedCue.value);
    duration.value = cueDuration;
  },
);

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  if (number.value != null) {
    selectedCue.value.number = number.value;
  }
  if (preWait.value != null) {
    selectedCue.value.preWait = preWait.value;
  }
  if (chain.value != null && props.chainOverride == null) {
    selectedCue.value.chain.type = chain.value;
    if (selectedCue.value.chain.type === 'doNotChain') {
      target.value = null;
    } else {
      selectedCue.value.chain.targetId = target.value != null ? target.value : null;
    }
  }
  if (name.value != null) {
    const newName = name.value.trim();
    if (newName === '') {
      selectedCue.value.name = null;
    } else {
      selectedCue.value.name = newName;
    }
  }
  if (notes.value != null) {
    selectedCue.value.notes = notes.value;
  }
  if (color.value != null) {
    selectedCue.value.color = color.value;
  }
  emit('update');
};

const insertTimestampToNote = () => {
  if (selectedCue.value == null) {
    return;
  }
  const position = showState.getPosition(selectedCue.value.id);
  if (position == null) return;

  if (notes.value == null) {
    notes.value = '';
  }

  if (notes.value.endsWith('\n') || notes.value === '') {
    notes.value += `[${secondsToFormat(position)}] `;
  } else {
    notes.value += `\n[${secondsToFormat(position)}] `;
  }
  saveEditorValue();
};
</script>

<template>
  <div class="flex flex-row p-3 gap-2 min-w-180">
    <div class="flex flex-col gap-2 grow-0 shrink-0 w-42">
      <text-input
        v-model="number"
        class="grow-0 text-center"
        :label="t('main.number')"
        @update="saveEditorValue"
      />
      <time-input
        v-model="duration"
        class="grow-0 text-center"
        :disabled="
          selectedCue != null &&
          selectedCue.params.type != 'wait' &&
          selectedCue.params.type != 'fade'
        "
        :label="t('main.duration')"
        @update="saveEditorValue"
      />
      <time-input
        v-model="preWait"
        class="grow-0 text-center"
        :label="t('main.preWait')"
        @update="saveEditorValue"
      />
      <select-wrapper
        v-model="chain"
        class="grow-0"
        :label="t('main.chainMode.title')"
        :disabled="
          (selectedCue != null && selectedCue.id in showState.activeCues) ||
          props.chainOverride != null
        "
        :items="[
          { value: 'doNotChain', name: t('main.chainMode.doNotChain') },
          { value: 'afterStart', name: t('main.chainMode.afterStart') },
          { value: 'afterComplete', name: t('main.chainMode.afterComplete') },
        ]"
        autocomplete="off"
        @update:model-value="saveEditorValue"
        @keydown.stop
      />
    </div>
    <div class="flex grow shrink flex-col gap-2">
      <text-input
        v-model="name"
        :placeholder="selectedCue != null ? buildCueName(selectedCue) : ''"
        :label="t('main.name')"
        align-input="left"
        class="grow-0"
        @update="saveEditorValue"
      />
      <textarea-wrapper
        v-model="notes"
        class="grow shrink"
        :label="t('main.notes')"
        text-type="area"
        @update="saveEditorValue"
      />
      <div class="flex flex-row grow-0 shrink-0 gap-3">
        <cue-select
          v-model="target"
          class="max-w-160 grow min-w-40"
          :label="t('main.bottomEditor.continueTargetCue')"
          cue-type="all"
          :exclude="selectedCue?.id"
          :null-text="t('main.bottomEditor.basics.nextCue')"
          :disabled="
            props.chainOverride != null ||
            selectedCue == null ||
            selectedCue.id in showState.activeCues ||
            chain == 'doNotChain'
          "
          @update="saveEditorValue"
        />
        <select-wrapper
          v-model="color"
          class="ml-auto grow-0"
          width="150px"
          :label="t('main.bottomEditor.basics.color')"
          :items="[
            { value: 'none', name: t('general.none'), color: 'text' },
            { value: 'red', name: 'Red', color: 'red' },
            { value: 'purple', name: 'Purple', color: 'purple' },
            { value: 'blue', name: 'Blue', color: 'blue' },
            { value: 'cyan', name: 'Cyan', color: 'cyan' },
            { value: 'green', name: 'Green', color: 'green' },
            { value: 'yellow', name: 'Yellow', color: 'yellow' },
            { value: 'orange', name: 'Orange', color: 'orange' },
            { value: 'grey', name: 'Grey', color: 'gray' }, // Backend uses 'grey' as key but primevue uses 'gray' as color name.
          ]"
          :prepend-inner-icon="mdiCircle"
          @update:model-value="saveEditorValue"
          @keydown.stop
        />
        <button-wrapper
          class="grow-0"
          :disabled="selectedCue != null && !(selectedCue.id in showState.activeCues)"
          :label="t('main.bottomEditor.basics.timestamp')"
          @click="insertTimestampToNote"
        />
      </div>
    </div>
  </div>
</template>
