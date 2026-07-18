<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

const props = defineProps<{
  level: number,
  type: 'item' | 'end',
}>();
</script>

<template>
        <svg
        v-if="props.level > 0"
        class="inline text-slate-400"
        xmlns="http://www.w3.org/2000/svg"
        :viewBox="`0 0 ${props.level * 16} 31`"
        preserveAspectRatio="none"
        height="100%"
        :width="`${props.level * 16}px`"
      >
        <rect v-for="i in (props.level - 1)" :key="i" :x="8 + (i - 1) * 16" y="0" width="1" height="32" fill="currentColor"></rect>
        <rect :x="8 + (props.level - 1) * 16" y="0" width="1" :height="props.type === 'item' ? 32 : 16" fill="currentColor"></rect>
        <rect :x="8 + (props.level - 1) * 16" y="16" width="8" height="1" fill="currentColor"></rect>
      </svg>
</template>