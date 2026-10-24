<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, onUnmounted } from 'vue';
import SliderWrapper from '../wrapper/SliderWrapper.vue';
import NumberInput from './NumberInput.vue';
import { debounce } from '@/utils.ts';

const props = defineProps<{
  label?: string;
  direction?: 'horizontal' | 'vertical';
  disabled?: boolean;
}>();

const panning = defineModel<number>({ default: 0 });
const emit = defineEmits(['update']);

let sliderChanging = false;
let pointerId: number | null = null;

const faderPosition = computed({
  get() {
    return panning.value * 64;
  },
  set(newValue) {
    panning.value = newValue / 64;
  },
});

const tickLabels = [
  { value: -64, label: 'L' },
  { value: 0, label: 'C' },
  { value: 64, label: 'R' },
];

const onPointerDown = (e: PointerEvent) => {
  sliderChanging = true;
  pointerId = e.pointerId;
  window.addEventListener('pointerup', onPointerUp);
};

const onPointerUp = (e: PointerEvent) => {
  if (pointerId !== e.pointerId) return;
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

onUnmounted(() => {
  window.removeEventListener('pointerup', onPointerUp);
});
</script>

<template>
  <slider-wrapper
    v-model="faderPosition"
    :step="1"
    :min="-64"
    :max="64"
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
    @pointerdown="onPointerDown"
    @keydown.stop
  >
    <template #input>
      <number-input
        v-model="panning"
        class="w-35"
        :disabled="props.disabled"
        :min="-1"
        :max="1"
        :step="1 / 8"
        :prefix="panning < 0 ? 'L ' : panning > 0 ? 'R ' : 'C '"
        :precision="3"
        @dblclick.stop
        @pointerdown.stop
        @update="emit('update')"
      />
    </template>
  </slider-wrapper>
</template>
