<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import Slider from 'primevue/slider';

const model = defineModel<number>();

const props = defineProps<{
  direction?: 'vertical' | 'horizontal';
  label?: string;
  min: number;
  max: number;
  step: number;
  ticks: { label: string; value: number }[];
}>();
</script>

<template>
  <template v-if="props.direction !== 'vertical'">
    <div class="flex flex-row items-center">
      <span>{{ props.label }}</span>
      <div class="flex grow flex-col px-8">
        <Slider
          v-model="model"
          :min="props.min"
          :max="props.max"
          :step="props.step"
          class="z-1"
        />
        <div class="relative mt-2 w-full">
          <span
            v-for="tick in props.ticks"
            :key="tick.value"
            class="pointer-events-none absolute transform-[translateX(-50%)]"
            :style="{
              left: `${((tick.value - props.min) * 100) / (props.max - props.min)}%`,
            }"
            >{{ tick.label }}</span
          >
        </div>
      </div>
      <div class="shrink-0 grow-0">
        <slot name="input"></slot>
      </div>
    </div>
  </template>
  <template v-else>
    <div class="flex flex-col items-center">
      <span>{{ props.label }}</span>
      <div class="flex grow flex-row py-4">
        <Slider
          v-model="model"
          :min="props.min"
          :max="props.max"
          :step="props.step"
          orientation="vertical"
          class="z-1"
        />
        <div class="relative ml-4 h-full">
          <span
            v-for="tick in props.ticks"
            :key="tick.value"
            class="pointer-events-none absolute transform-[translateY(50%)]"
            :style="{
              bottom: `${((tick.value - props.min) * 100) / (props.max - props.min)}%`,
            }"
            >{{ tick.label }}</span
          >
        </div>
      </div>
      <div class="shrink-0 grow-0">
        <slot name="input"></slot>
      </div>
    </div>
  </template>
</template>
