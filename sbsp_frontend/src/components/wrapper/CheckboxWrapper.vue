<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import Checkbox from 'primevue/checkbox';
import { useId } from 'vue';

const model = defineModel<boolean>();
const props = defineProps<{
  label?: string;
  disabled?: boolean;
}>();

const inputId = useId();
</script>

<template>
  <div class="flex items-center gap-2">
    <Checkbox
      v-model="model"
      :inputId="inputId"
      :disabled="props.disabled"
      binary
    />
    <label :for="inputId">{{ props.label || '' }}</label>
  </div>
</template>
