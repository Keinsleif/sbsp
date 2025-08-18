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
import ToolHeader from './components/ToolHeader.vue';
import CueList from './components/CueList.vue';
import SideBar from './components/SideBar.vue';
import FootBar from './components/FootBar.vue';
import { useUiState } from './stores/uistate';
import BottomEditor from './components/BottomEditor.vue';

const uiState = useUiState();
</script>

<style>
html {
  overflow-y: hidden;
  overscroll-behavior-y: none;
  user-select: none;
}
</style>
