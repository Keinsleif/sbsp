<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

const props = defineProps<{
  icon: string | null;
}>();
</script>

<template>
  <svg
    class="inline"
    xmlns="http://www.w3.org/2000/svg"
    viewBox="2 2 22 22"
    width="16"
    height="16"
  >
    <path
      :d="props.icon || ''"
      fill="currentColor"
    />
  </svg>
</template>
