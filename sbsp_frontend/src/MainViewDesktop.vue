<template>
  <v-app
    height="100vh"
    @contextmenu.prevent
  >
    <v-app-bar
      app
      border
      flat
      height="200"
    >
      <ToolHeader />
    </v-app-bar>

    <v-main style="height: 100vh">
      <v-sheet class="d-flex h-100">
        <CueList />
      </v-sheet>
    </v-main>

    <v-footer
      app
      border
      class="py-1"
    >
      <FootBar />
    </v-footer>

    <v-navigation-drawer
      :model-value="uiState.isRightSidebarOpen && mdAndUp"
      app
      permanent
      persistent
      touchless
      location="right"
      width="260"
    >
      <SideBar />
    </v-navigation-drawer>

    <v-navigation-drawer
      :model-value="uiState.isBottomTabOpen && uiState.mode == 'edit'"
      app
      permanent
      persistent
      touchless
      location="bottom"
      width="250"
    >
      <BottomEditor
        v-model="selectedCue"
        :chain-override="selectedCueChainOverride"
        @update="onCueEdited"
      />
    </v-navigation-drawer>

    <v-snackbar-queue
      v-model="uiState.success_messages"
      timeout="2000"
      color="success"
    />
    <v-snackbar-queue
      v-model="uiState.error_messages"
      timeout="2000"
      color="error"
    />

    <renumber-dialog v-model="uiState.isRenumberCueDialogOpen" />
    <settings-dialog v-model="uiState.isSettingsDialogOpen" />
    <file-list-dialog
      v-if="side == 'remote'"
      v-model="uiState.fileListResolver"
      :multiple="uiState.fileListOption"
    />
    <server-panel-dialog
      v-if="side == 'host'"
      v-model="uiState.isServerPanelOpen"
    />
  </v-app>
</template>

<script setup lang="ts">
import ToolHeader from './components/pc/ToolHeader.vue';
import CueList from './components/pc/CueList.vue';
import SideBar from './components/pc/SideBar.vue';
import FootBar from './components/pc/FootBar.vue';
import BottomEditor from './components/pc/BottomEditor.vue';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';
import RenumberDialog from './components/dialog/RenumberDialog.vue';
import SettingsDialog from './components/dialog/SettingsDialog.vue';
import FileListDialog from './components/dialog/FileListDialog.vue';
import ServerPanelDialog from './components/dialog/ServerPanelDialog.vue';
import { useApi, side } from './api';
import { useDisplay } from 'vuetify/lib/composables/display.mjs';
import { storeToRefs } from 'pinia';
import { computed, ref, toRaw, watch } from 'vue';
import type { Cue } from './types/Cue';
import { debounce } from './utils';

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const uiState = useUiState();
const api = useApi();
const { mdAndUp } = useDisplay();

const selectedCue = ref<Cue | null>(uiState.selected != null ? getCueById.value(uiState.selected)! : null);
const selectedCueChainOverride = computed(() => {
  if (selectedCue.value == null) {
    return null;
  }
  const flatEntry = showModel.flatCueList.find(item => item.cue.id == selectedCue.value!.id);
  if (flatEntry == null) {
    return null;
  }
  if (flatEntry.isChainOverrided) {
    return flatEntry.chain;
  } else {
    return null;
  }
});

watch(
  () => uiState.selected,
  () => {
    if (onCueEdited.debouncing) {
      onCueEdited.clear();
      onCueEdited.immediate();
    }
    selectedCue.value = uiState.selected != null ? getCueById.value(uiState.selected)! : null;
  },
);

watch(() => showModel.cues, () => {
  selectedCue.value = uiState.selected != null ? getCueById.value(uiState.selected)! : null;
});

const onCueEdited = debounce(() => {
  if (selectedCue.value == null) {
    return;
  }
  api.updateCue(toRaw(selectedCue.value));
}, 200);

</script>
