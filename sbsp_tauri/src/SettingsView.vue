<template>
  <div class="d-flex flex-column w-100 h-100">
    <v-sheet class="flex-grow-1 d-flex flex-row w-100">
      <v-tabs v-model="tab" direction="vertical">
        <v-tab text="General" value="general"></v-tab>
        <v-tab text="Hotkey" value="hotkey"></v-tab>
      </v-tabs>
      <v-tabs-window v-model="tab" class="flex-grow-1">
        <v-tabs-window-item
          value="general"
          transition="slide-y-transition"
          reverse-transition="slide-y-reverse-transition"
        >
          <v-checkbox
            v-model="editingSettings.general.lockCursorToSelection"
            label="Lock Cursor to Selection"
          ></v-checkbox>
        </v-tabs-window-item>
        <v-tabs-window-item
          value="hotkey"
          class="pa-3"
          transition="slide-y-transition"
          reverse-transition="slide-y-reverse-transition"
        >
          <v-text-field
            v-model="editingSettings.hotkey.go"
            readonly
            clearable
            persistent-clear
            variant="outlined"
            density="compact"
            label="Go"
            @keydown="inputHotkey('go', $event)"
          ></v-text-field>
          <v-text-field
            v-model="editingSettings.hotkey.load"
            readonly
            clearable
            persistent-clear
            variant="outlined"
            density="compact"
            label="Load"
            @keydown="inputHotkey('load', $event)"
          ></v-text-field>
          <v-text-field
            v-model="editingSettings.hotkey.stop"
            readonly
            clearable
            persistent-clear
            variant="outlined"
            density="compact"
            label="Stop"
            @keydown="inputHotkey('stop', $event)"
          ></v-text-field>
          <v-text-field
            v-model="editingSettings.hotkey.stopAll"
            readonly
            clearable
            persistent-clear
            variant="outlined"
            density="compact"
            label="Stop All"
            @keydown="inputHotkey('stopAll', $event)"
          ></v-text-field>
        </v-tabs-window-item>
      </v-tabs-window>
    </v-sheet>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100">
      <v-btn class="ml-auto" text="Cancel" @click="getCurrentWindow().close()"></v-btn>
      <v-btn
        text="Done"
        @click="
          saveSettings();
          getCurrentWindow().close();
        "
      ></v-btn>
    </v-footer>
  </div>
</template>

<script setup lang="ts">
import { ref, toRaw, watch } from 'vue';
import { useShowModel } from './stores/showmodel';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { ShowSettings } from './types/ShowSettings';

const showModel = useShowModel();

const tab = ref('general');
const editingSettings = ref<ShowSettings>(structuredClone(toRaw(showModel.settings)));

watch(
  () => showModel.settings,
  (newSettings) => {
    editingSettings.value = structuredClone(toRaw(newSettings));
  },
);

const saveSettings = () => {
  invoke('update_settings', { newSettings: editingSettings.value }).catch((e) => console.error(e));
};

const inputHotkey = (action: string, event: KeyboardEvent) => {
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
  switch (action) {
    case 'go':
      editingSettings.value.hotkey.go = shortcut;
      break;
    case 'load':
      editingSettings.value.hotkey.load = shortcut;
      break;
    case 'stop':
      editingSettings.value.hotkey.stop = shortcut;
      break;
    case 'stopAll':
      editingSettings.value.hotkey.stopAll = shortcut;
      break;
  }
};
</script>
