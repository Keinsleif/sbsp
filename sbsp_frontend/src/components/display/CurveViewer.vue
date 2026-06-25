<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    power?: string | number | null;
    curve?: 'linear' | 'inPow' | 'outPow' | 'inOutPow' | null;
    type?: 'in' | 'out';
    strokeWidth?: number;
    disabled?: boolean;
  }>(),
  {
    power: 1,
    curve: 'linear',
    type: 'in',
    strokeWidth: 2,
    disabled: false,
  },
);

const command = computed(() => {
  if (props.type == null || props.disabled) {
    return '';
  }
  let result;
  if (props.type === 'in') {
    result = 'M 0 100';
  } else {
    result = 'M 0 0';
  }
  for (let i = 1; i < 100; i++) {
    let y = 0;
    if (props.curve === 'linear') {
      y = i / 100;
    } else if (props.curve === 'inPow') {
      y = Math.pow(i / 100, Number(props.power));
    } else if (props.curve === 'outPow') {
      y = 1 - Math.pow(1 - i / 100, Number(props.power));
    } else if (props.curve === 'inOutPow') {
      if (i < 50) {
        y = 0.5 * Math.pow(i / 50, Number(props.power));
      } else {
        y = 0.5 * (1 - Math.pow(2 - i / 50, Number(props.power))) + 0.5;
      }
    }
    if (props.type === 'in') {
      result += ` L ${i} ${100 - y * 100}`;
    } else {
      result += ` L ${i} ${y * 100}`;
    }
  }
  return result;
});
</script>

<template>
  <svg
    preserveAspectRatio="none"
    xmlns="http://www.w3.org/2000/svg"
    viewBox="-1 -1 101 101"
  >
    <path
      d="M10 0v100M20 0v100M30 0v100M40 0v100M50 0v100M60 0v100M70 0v100M80 0v100M90 0v100M0 10h100M0 20h100M0 30h100M0 40h100M0 50h100M0 60h100M0 70h100M0 80h100M0 90h100"
      stroke="rgb(from var(--p-surface-500) r g b / 0.5)"
    />
    <path
      :d="command"
      fill="none"
      :stroke-width="props.strokeWidth"
      stroke="var(--p-primary-500)"
    />
    <rect
      x="0"
      y="0"
      width="100"
      height="100"
      stroke="var(--p-surface-500)"
      fill="transparent"
    />
  </svg>
</template>
