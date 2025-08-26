<template>
  <v-text-field
    v-model="hotkey"
    readonly
    clearable
    persistent-clear
    variant="outlined"
    density="compact"
    @keydown="keyinput($event)"
  ></v-text-field>
</template>

<script setup lang="ts">
const hotkey = defineModel<string | null>({ default: '' });

const keyinput = (event: KeyboardEvent) => {
  event.preventDefault();
  let shortcut = '';
  if (event.ctrlKey) {
    shortcut += 'Ctrl+';
  }
  if (event.altKey) {
    shortcut += 'Alt+';
  }
  if (event.shiftKey) {
    shortcut += 'Shift+';
  }
  if (event.key == 'Control') {
    shortcut = 'Ctrl';
  } else if (event.key == 'Alt') {
    shortcut = 'Alt';
  } else if (event.key == 'Shift') {
    shortcut = 'Shift';
  } else if (event.key == ' ') {
    shortcut += 'Space';
  } else if (event.key.length === 1) {
    shortcut += event.key.toUpperCase();
  } else {
    shortcut += event.key;
  }
  hotkey.value = shortcut;
};
</script>
