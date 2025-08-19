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

const uiState = useUiState();
const showModel = useShowModel();

useHotkey(showModel.settings.hotkey.go != null ? showModel.settings.hotkey.go : undefined, () => {
  invoke('go').catch((e) => console.error(e));
});
</script>
