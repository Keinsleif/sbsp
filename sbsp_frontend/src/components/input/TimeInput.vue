<template>
  <v-text-field
    v-model="formattedValue"
    hide-details
    persistent-placeholder
    variant="outlined"
    density="compact"
    :class="$style['centered-input']"
    autocomplete="off"
    @blur="save"
    @keydown.stop="onKeydown"
  />
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import { formatToSeconds, secondsToFormat } from '../../utils';

const seconds = defineModel<number | null>({ default: null });
const props = withDefaults(
  defineProps<{
    acceptMinus?: boolean;
    multiply?: number;
    max?: number | null;
    defaultValue?: number;
  }>(),
  {
    max: null,
    multiply: 1,
    acceptMinus: false,
    defaultValue: 0,
  },
);
const emit = defineEmits(['update']);

const formattedValue = ref(secondsToFormat(seconds.value != null ? seconds.value * props.multiply : null));

watch([seconds, () => props.multiply], () => {
  formattedValue.value = secondsToFormat(seconds.value != null ? seconds.value * props.multiply : null);
});

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && e.target instanceof HTMLElement) {
    e.target.blur();
    return;
  }
  if (e.key === 'Escape' && e.target instanceof HTMLElement) {
    formattedValue.value = secondsToFormat(seconds.value != null ? seconds.value * props.multiply : null); // reset
    e.target.blur();
  }
};

const save = () => {
  let innerValue: number;
  if (formattedValue.value.trim() === '') {
    innerValue = props.defaultValue;
  } else {
    innerValue = formatToSeconds(formattedValue.value, props.acceptMinus) / props.multiply;
  }
  if (props.max != null && innerValue > props.max) {
    innerValue = props.max;
  }
  if (seconds.value !== innerValue) {
    seconds.value = innerValue;
    emit('update');
  }
};
</script>

<style lang="css" module>
  .centered-input input {
    text-align: center;
  }
</style>
