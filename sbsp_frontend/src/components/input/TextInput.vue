<template>
  <v-text-field
    v-if="props.textType == 'single'"
    v-bind="$attrs"
    v-model="innerText"
    :hide-details="!props.showDetails"
    persistent-placeholder
    variant="outlined"
    density="compact"
    autocomplete="off"
    :class="
      props.alignInput == 'left'
        ? $style['left-input']
        : props.alignInput == 'center'
          ? $style['center-input']
          : $style['right-input']
    "
    @blur="save"
    @keydown.stop="onKeydown"
  />
  <v-textarea
    v-else-if="props.textType == 'area'"
    v-bind="$attrs"
    v-model="innerText"
    :hide-details="!props.showDetails"
    persistent-placeholder
    variant="outlined"
    density="compact"
    autocomplete="off"
    rows="1"
    auto-grow
    no-resize
    @blur="save"
    @keydown.stop="onKeydown"
  />
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { nextTick, ref, watch } from 'vue';

const text = defineModel<string | null>({ default: '' });
const props = withDefaults(
  defineProps<{
    textType?: 'single' | 'area';
    alignInput?: 'left' | 'center' | 'right';
    showDetails?: boolean;
  }>(),
  {
    textType: 'single',
    alignInput: 'center',
    showDetails: false,
  },
);
const emit = defineEmits(['update']);

const innerText = ref(text.value ?? '');

watch(text, () => {
  innerText.value = text.value ?? '';
});

const save = () => {
  if (text.value !== innerText.value) {
    text.value = innerText.value;
    emit('update');
  }
};

const onKeydown = (e: KeyboardEvent) => {
  if (!(e.target instanceof HTMLElement)) return;
  switch (e.key) {
    case 'Enter':
      if (props.textType === 'single') {
        e.target.blur();
      }
      break;
    case 'Escape':
      innerText.value = text.value != null ? text.value : ''; // reset
      e.target.blur();
      break;
    case 'Tab':
      if (e.target instanceof HTMLTextAreaElement) {
        e.preventDefault();
        const textarea = e.target;
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;

        const value = innerText.value;
        innerText.value = value.substring(0, start) + '\t' + value.substring(end);

        nextTick(() => {
          textarea.selectionStart = textarea.selectionEnd = start + 1;
        });
        break;
      }
  }
};
</script>

<style lang="css" module>
  .center-input input {
    text-align: center;
  }
  .left-input input {
    text-align: left;
  }
  .right-input input {
    text-align: right;
  }
</style>
