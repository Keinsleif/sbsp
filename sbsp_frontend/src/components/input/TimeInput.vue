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
    @keydown.enter="$event.target.blur()"
    @keydown.esc="
      reset();
      $event.target.blur();
    "
    @keydown.stop
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

const save = () => {
  let innerValue: number;
  if (formattedValue.value.trim() == '') {
    innerValue = props.defaultValue;
  } else {
    innerValue = formatToSeconds(formattedValue.value, props.acceptMinus) / props.multiply;
  }
  if (props.max != null && innerValue > props.max) {
    innerValue = props.max;
  }
  if (seconds.value != innerValue) {
    seconds.value = innerValue;
    emit('update');
  }
};

const reset = () => {
  formattedValue.value = secondsToFormat(seconds.value != null ? seconds.value * props.multiply : null);
};
</script>

<style lang="css" module>
  .centered-input input {
    text-align: center;
  }
</style>
