<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiCheck, mdiContentCopy } from '@mdi/js';
import { ref, useId } from 'vue';
import InputGroupAddon from 'primevue/inputgroupaddon';
import InputGroup from 'primevue/inputgroup';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';

defineOptions({ inheritAttrs: false });
const value = defineModel<string | null>({ default: '' });
const props = defineProps<{
  label?: string;
}>();

const copied = ref(false);

const copyToClipboard = () => {
  if (value.value) {
    copied.value = true;
    navigator.clipboard.writeText(value.value);
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  }
};

const inputId = useId();
</script>

<template>
  <input-group>
    <float-label variant="on">
      <input-text
        v-model="value"
        v-bind="$attrs"
        class="h-full w-full"
        :id="inputId"
        autocomplete="off"
        :pt="{
          root: () => {
            return {
              style: 'background-color: var(--p-inputtext-background);',
            };
          },
        }"
      />
      <label :for="inputId">{{ props.label || '' }}</label>
    </float-label>
    <input-group-addon>
      <button-wrapper
        :icon="copied ? mdiCheck : mdiContentCopy"
        severity="secondary"
        variant="text"
        @click="copyToClipboard"
      />
    </input-group-addon>
  </input-group>
</template>
