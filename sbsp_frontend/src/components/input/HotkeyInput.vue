<script setup lang="ts">
import { mdiClose } from '@mdi/js';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';

// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

const hotkey = defineModel<string | null>({ default: '' });

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
</script>

<template>
  <float-label variant="on">
    <input-text
      id="on_label"
      class="mt-4 w-125"
      v-model="hotkey"
      readonly
      autocomplete="off"
      @keydown.stop="keyinput($event)"
    />
    <label for="on_label">t('dialog.settings.show.general.assetsDirectory.title')</label>
    <button-wrapper
      :icon="mdiClose"
      @click="hotkey = null"
    ></button-wrapper>
  </float-label>
</template>
