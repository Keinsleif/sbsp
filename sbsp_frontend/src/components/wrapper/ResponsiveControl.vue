<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import Dialog from 'primevue/dialog';
import { ref } from 'vue';
import ButtonWrapper from './ButtonWrapper.vue';

const isVisible = ref(false);

const controlProps = withDefaults(
  defineProps<{
    overlay?: boolean;
    buttonLabel?: string;
  }>(),
  {
    overlay: false,
    buttonLabel: 'Open',
  },
);
</script>

<template>
  <template v-if="!controlProps.overlay">
    <slot />
  </template>
  <template v-else>
    <button-wrapper
      :label="controlProps.buttonLabel"
      @click="isVisible = true"
    />
    <Dialog v-model:visible="isVisible">
      <slot />
    </Dialog>
  </template>
</template>
