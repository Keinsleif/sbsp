<template>
  <v-app height="100vh">
    <v-app-bar app border flat height="200">
      <ToolHeader />
    </v-app-bar>

    <v-main style="height: 100vh">
      <v-sheet class="d-flex fill-height list-wrapper">
        <CueList />
      </v-sheet>
    </v-main>

    <v-footer app border class="py-1">
      <FootBar />
    </v-footer>

    <v-navigation-drawer v-model="uiState.isRightSidebarOpen" app permanent location="right" width="300">
      <SideBar />
    </v-navigation-drawer>

    <v-navigation-drawer v-model="uiState.isEditorOpen" app permanent location="bottom" width="301">
      <BottomEditor />
    </v-navigation-drawer>

    <v-snackbar-queue v-model="uiState.success_messages" timeout="2000" color="success"></v-snackbar-queue>
    <v-snackbar-queue v-model="uiState.error_messages" timeout="2000" color="error"></v-snackbar-queue>
  </v-app>
</template>

<script setup lang="ts">
import { useHotkey } from 'vuetify';
import { invoke } from '@tauri-apps/api/core';
import ToolHeader from './components/ToolHeader.vue';
import CueList from './components/CueList.vue';
import SideBar from './components/SideBar.vue';
import FootBar from './components/FootBar.vue';
import BottomEditor from './components/BottomEditor.vue';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';
import { onMounted, onUnmounted, ref } from 'vue';

const uiState = useUiState();
const showModel = useShowModel();

const wakeLock = ref<WakeLockSentinel | null>(null);

const onVisibilityChange = () => {
  if (wakeLock.value !== null && document.visibilityState === 'visible') {
    navigator.wakeLock.request('screen').then((value) => {
      wakeLock.value = value;
    });
  }
};

onMounted(() => {
  navigator.wakeLock.request('screen').then((value) => {
    wakeLock.value = value;
  });
  document.addEventListener('visibilitychange', onVisibilityChange);
});

onUnmounted(() => {
  document.removeEventListener('visibilitychange', onVisibilityChange);
  if (wakeLock.value != null) {
    wakeLock.value.release().then(() => {
      wakeLock.value = null;
    });
  }
});

useHotkey(showModel.settings.hotkey.go != null ? showModel.settings.hotkey.go : undefined, () => {
  invoke('go').catch((e) => console.error(e));
});
useHotkey(showModel.settings.hotkey.load != null ? showModel.settings.hotkey.load : undefined, () => {
  invoke('load').catch((e) => console.error(e));
});
useHotkey(showModel.settings.hotkey.stop != null ? showModel.settings.hotkey.stop : undefined, () => {
  invoke('stop').catch((e) => console.error(e));
});
</script>
