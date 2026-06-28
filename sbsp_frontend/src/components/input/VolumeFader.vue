<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref } from 'vue';
import VolumeInput from './VolumeInput.vue';
import { debounce, decibelsToFader, faderToDecibels } from '../../utils';
import SliderWrapper from '../wrapper/SliderWrapper.vue';

const props = withDefaults(
  defineProps<{
    label?: string;
    direction?: 'horizontal' | 'vertical';
    thumbAmount?: 'full' | 'decreased' | 'baseOnly';
    disabled?: boolean;
  }>(),
  {
    label: 'Volume',
    direction: 'horizontal',
    thumbAmount: 'full',
  },
);

const volume = defineModel<number>({ default: 0 });
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const faderPosition = computed({
  get() {
    return decibelsToFader(volume.value);
  },
  set(newValue) {
    volume.value = faderToDecibels(newValue);
  },
});

const onPointerUp = debounce(() => {
  if (sliderChanging.value) {
    sliderChanging.value = false;
    if (!props.disabled) {
      emit('update');
    }
  }
}, 300);

const tickLabels = computed(() => {
  if (props.thumbAmount === 'decreased') {
    return [
      { value: 10, label: '10' },
      { value: 0, label: '0' },
      { value: -10, label: '-10' },
      { value: -30, label: '-60' },
    ];
  } else if (props.thumbAmount === 'baseOnly') {
    return [{ value: 0, label: '0' }];
  } else {
    return [
      { value: 10, label: '10' },
      { value: 5, label: '5' },
      { value: 0, label: '0' },
      { value: -5, label: '-5' },
      { value: -10, label: '-10' },
      { value: -15, label: '-20' },
      { value: -20, label: '-30' },
      { value: -25, label: '-40' },
      { value: -30, label: '-60' },
    ];
  }
});
</script>

<template>
  <slider-wrapper
    v-model="faderPosition"
    :step="0.05"
    :min="-30"
    :max="10"
    :label="props.label"
    :ticks="tickLabels"
    :direction="props.direction"
    :disabled="props.disabled"
    @dblclick="
      if (!props.disabled) {
        faderPosition = 0;
        onPointerUp.clear();
        emit('update');
      }
    "
    @keydown.stop
    @pointerdown="sliderChanging = true"
    @pointerup="onPointerUp"
  >
    <template #input>
      <volume-input
        v-model="volume"
        :disabled="props.disabled"
        @pointerdown.stop
        @dblclick.stop
        @update="emit('update')"
      />
    </template>
  </slider-wrapper>
</template>
