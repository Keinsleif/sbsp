<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import ProgressSpinner from 'primevue/progressspinner';
import { $dt } from '@primeuix/themes';

const props = withDefaults(
  defineProps<{
    size?: string;
    color?: string;
  }>(),
  {
    size: '16px',
    color: 'text.color',
  },
);
</script>

<template>
  <ProgressSpinner
    stroke-width="10"
    animation-duration="1s"
    :style="{
      width: props.size,
      height: props.size,
    }"
    :pt="{
      circle: {
        style: `animation: none; stroke-dasharray: 25 200; stroke: ${$dt(props.color).variable};`,
      },
    }"
  />
</template>
