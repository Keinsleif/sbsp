<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, onUnmounted } from 'vue';
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

let sliderChanging = false;
let pointerId: number | null = null;

const faderPosition = computed({
  get() {
    return decibelsToFader(volume.value);
  },
  set(newValue) {
    volume.value = faderToDecibels(newValue);
  },
});

const onPointerDown = (e: PointerEvent) => {
  sliderChanging = true;
  pointerId = e.pointerId;
  window.addEventListener('pointerup', onPointerUp);
};

const onPointerUp = (e: PointerEvent) => {
  if (e.pointerId !== pointerId) return;
  window.removeEventListener('pointerup', onPointerUp);
  flushFader();
  pointerId = null;
};

const flushFader = debounce(() => {
  if (sliderChanging) {
    sliderChanging = false;
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

onUnmounted(() => {
  window.removeEventListener('pointerup', onPointerUp);
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
        flushFader.clear();
        emit('update');
      }
    "
    @keydown.stop
    @pointerdown="onPointerDown"
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
