<template>
  <div class="d-flex flex-column w-100 h-100">
    <v-sheet class="flex-grow-1 d-flex flex-row w-100">
      <v-tabs v-model="tab" direction="vertical">
        <v-tab text="General" value="general"></v-tab>
        <v-tab text="Hotkey" value="hotkey"></v-tab>
        <v-tab text="Audio" value="audio"></v-tab>
      </v-tabs>
      <v-tabs-window v-model="tab" class="flex-grow-1">
        <v-tabs-window-item
          value="general"
          transition="toggle-slide-y-transition"
          reverse-transition="toggle-slide-y-reverse-transition"
        >
          <v-checkbox v-model="editingSettings.general.advanceCursorWhenGo" label="Advance cursor when GO"></v-checkbox>
          <v-divider></v-divider>
          <v-checkbox
            v-model="editingSettings.general.lockCursorToSelection"
            label="Lock Cursor to Selection"
          ></v-checkbox>
        </v-tabs-window-item>
        <v-tabs-window-item
          value="hotkey"
          class="pa-3"
          transition="toggle-slide-y-transition"
          reverse-transition="toggle-slide-y-reverse-transition"
        >
          <h2>Playback</h2>
          <hotkey-input v-model="editingSettings.hotkey.playback.go" label="Go"></hotkey-input>
          <hotkey-input v-model="editingSettings.hotkey.playback.load" label="Load"></hotkey-input>
          <hotkey-input v-model="editingSettings.hotkey.playback.pauseAndResume" label="Pause & Resume"></hotkey-input>
          <hotkey-input v-model="editingSettings.hotkey.playback.pauseAll" label="Pause All"></hotkey-input>
          <hotkey-input v-model="editingSettings.hotkey.playback.resumeAll" label="Resume All"></hotkey-input>
          <hotkey-input v-model="editingSettings.hotkey.playback.stop" label="Stop"></hotkey-input>
          <hotkey-input v-model="editingSettings.hotkey.playback.stopAll" label="Stop All"></hotkey-input>
          <v-divider></v-divider>
          <h2>Audio Action</h2>
          <hotkey-input v-model="editingSettings.hotkey.audioAction.toggleRepeat" label="ToggleRepeat"></hotkey-input>
        </v-tabs-window-item>
        <v-tabs-window-item
          value="audio"
          class="pa-3"
          transition="toggle-slide-y-transition"
          reverse-transition="toggle-slide-y-reverse-transition"
        >
          <v-checkbox v-model="editingSettings.audio.monoOutput" label="Downmix stereo to mono"></v-checkbox>
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
import HotkeyInput from './components/input/HotkeyInput.vue';

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
</script>
