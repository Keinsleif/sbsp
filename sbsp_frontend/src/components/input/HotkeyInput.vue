<script setup lang="ts">
import { mdiClose } from '@mdi/js';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import InputGroup from 'primevue/inputgroup';
import InputGroupAddon from 'primevue/inputgroupaddon';
import { useId } from 'vue';
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';

// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

const hotkey = defineModel<string | null>({ default: '' });
const props = defineProps<{
  label?: string;
}>();

const keyinput = (event: KeyboardEvent) => {
  event.preventDefault();
  let shortcut = '';
  if (event.ctrlKey) {
    shortcut += 'Ctrl+';
  }
  if (event.metaKey) {
    shortcut += 'Cmd+';
  }
  if (event.altKey) {
    shortcut += 'Alt+';
  }
  if (event.shiftKey) {
    shortcut += 'Shift+';
  }
  if (event.key === 'Control') {
    shortcut = 'Ctrl';
  } else if (event.key === 'Meta' || event.key === 'OS') {
    shortcut = 'Cmd';
  } else if (event.key === 'Alt') {
    shortcut = 'Alt';
  } else if (event.key === 'Shift') {
    shortcut = 'Shift';
  } else if (event.key === ' ') {
    shortcut += 'Space';
  } else if (event.key.length === 1) {
    shortcut += event.key.toUpperCase();
  } else {
    shortcut += event.key;
  }
  hotkey.value = shortcut;
};

const inputId = useId();
</script>

<template>
  <input-group>
    <float-label
      variant="on"
      class="w-125"
    >
      <input-text
        v-model="hotkey"
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
        @keydown.stop="keyinput($event)"
      />
      <label :for="inputId">{{ props.label || '' }}</label>
    </float-label>
    <input-group-addon>
      <button-wrapper
        :icon="mdiClose"
        severity="secondary"
        variant="text"
        @click="hotkey = null"
      />
    </input-group-addon>
  </input-group>
</template>
