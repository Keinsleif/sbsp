<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import Checkbox from 'primevue/checkbox';
import { useId } from 'vue';

defineOptions({ inheritAttrs: false });
const props = defineProps<{
  label?: string;
}>();

const inputId = useId();
</script>

<template>
  <div class="flex items-center gap-2">
    <Checkbox
      v-bind="$attrs"
      :inputId="inputId"
      binary
    />
    <label :for="inputId">{{ props.label || '' }}</label>
  </div>
</template>
