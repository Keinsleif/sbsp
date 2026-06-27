<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import FloatLabel from 'primevue/floatlabel';
import InputNumber from 'primevue/inputnumber';
import { useId } from 'vue';

defineOptions({ inheritAttrs: false });
const props = defineProps<{
  label?: string;
}>();

const inputId = useId();
</script>

<template>
  <float-label variant="on">
    <input-number
      :input-id="inputId"
      class="w-25"
      v-bind="$attrs"
      :pt="{
        pcInputText: {
          root: {
            class: 'w-25',
          },
        },
      }"
    />
    <label :for="inputId">{{ props.label || '' }}</label>
  </float-label>
</template>
