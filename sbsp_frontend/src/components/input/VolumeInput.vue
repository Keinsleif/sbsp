<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import InputText from 'primevue/inputtext';
import { ref, watch } from 'vue';

const volume = defineModel<number>();
const emit = defineEmits(['update']);

const validateVolume = (src: number | undefined): string => {
  if (src == null) {
    return '0.00';
  }
  if (src > 10) {
    return '10.00';
  } else if (src <= -60) {
    return '-∞';
  } else {
    return src.toFixed(2);
  }
};

const innerVolume = ref(validateVolume(volume.value));

watch(volume, () => {
  innerVolume.value = validateVolume(volume.value);
});

const onKeydown = (e: KeyboardEvent) => {
  if (!(e.target instanceof HTMLElement)) return;
  if (e.key === 'Enter') {
    e.target.blur();
  } else if (e.key === 'Escape') {
    innerVolume.value = validateVolume(volume.value);
    e.target.blur();
  }
};

const saveValue = () => {
  const newVolume = Number(innerVolume.value);
  if (isNaN(newVolume)) {
    innerVolume.value = validateVolume(volume.value);
    return;
  }
  volume.value = newVolume;
  emit('update');
};
</script>

<template>
  <InputText
    v-model="innerVolume"
    v-bind="$attrs"
    class="w-25"
    autocomplete="off"
    suffix=" dB"
    @blur="saveValue"
    @keydown.stop="onKeydown"
  />
</template>
