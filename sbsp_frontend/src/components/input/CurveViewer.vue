<template>
  <svg preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg" viewBox="-1 -1 101 101">
    <rect width="1" height="100" x="10" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="20" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="30" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="40" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="50" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="60" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="70" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="80" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="1" height="100" x="90" y="0" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="10" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="20" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="30" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="40" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="50" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="60" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="70" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="80" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <rect width="100" height="1" x="0" y="90" fill="rgb(var(--v-theme-surface-variant), 0.3)"></rect>
    <path :d="command" fill="none" :stroke-width="props.strokeWidth" stroke="rgb(var(--v-theme-primary))"></path>
  </svg>
</template>

<script setup lang="ts">
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
    if (props.type == 'in') {
      result = 'M 0 100';
    } else {
      result = 'M 0 0';
    }
    for (let i = 1; i < 100; i++) {
      let y = 0;
      if (props.curve == 'linear') {
        y = i / 100;
      } else if (props.curve == 'inPow') {
        y = Math.pow(i / 100, Number(props.power));
      } else if (props.curve == 'outPow') {
        y = 1 - Math.pow(1 - i / 100, Number(props.power));
      } else if (props.curve == 'inOutPow') {
        if (i < 50) {
          y = 0.5 * Math.pow(i / 50, Number(props.power));
        } else {
          y = 0.5 * (1 - Math.pow(2 - i / 50, Number(props.power))) + 0.5;
        }
      }
      if (props.type == 'in') {
        result += ` L ${i} ${100 - y * 100}`;
      } else {
        result += ` L ${i} ${y * 100}`;
      }
    }
    return result;
  });
</script>
