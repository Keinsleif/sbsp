<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref } from 'vue';
import SliderWrapper from '../wrapper/SliderWrapper.vue';
import InputNumber from 'primevue/inputnumber';

const props = defineProps<{
  label?: string;
  direction?: 'horizontal' | 'vertical';
}>();

const panning = defineModel<number>({ default: 0 });
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

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
    @dblclick="faderPosition = 0"
    @pointerdown="sliderChanging = true"
    @pointerup="
      if (sliderChanging) {
        sliderChanging = false;
        emit('update');
      }
    "
    @keydown.stop
  >
    <template #input>
      <input-number
        v-model="panning"
        class="w-25"
        :min="-1"
        :max="1"
        :step="1 / 8"
        :prefix="panning < 0 ? 'L ' : panning > 0 ? 'R ' : 'C '"
        :max-fraction-digits="3"
        :pt="{
          pcInputText: {
            root: {
              class: 'w-25',
            },
          },
        }"
        @pointerdown.stop
        @dblclick.stop
      />
    </template>
  </slider-wrapper>
</template>
