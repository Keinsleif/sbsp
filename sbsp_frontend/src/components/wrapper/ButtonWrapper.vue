<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import Button from 'primevue/button';
import { $dt } from '@primeuix/themes';

const props = defineProps<{
  icon?: string;
  activeColor?: string;
  blink?: boolean;
  active?: boolean;
}>();
</script>

<template>
  <Button
    :style="{
      '--active-color': props.activeColor ? $dt(props.activeColor).variable : '',
    }"
    :class="[props.active ? $style['active'] : '', props.blink ? $style['blink'] : '']"
  >
    <template #icon="innerProps">
      <svg
        v-if="props.icon != null"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="2 2 22 22"
        width="16"
        height="16"
        :class="innerProps.class"
      >
        <path
          :d="props.icon"
          fill="currentColor"
        />
      </svg>
    </template>
  </Button>
</template>

<style lang="css" module>
.active {
  background-color: var(--active-color);
}
.blink {
  animation: flash 1s ease infinite;
}
@keyframes flash {
  0%,
  100% {
    background-color: var(--active-color);
  }
  50% {
    background-color: transparent;
  }
}
</style>
