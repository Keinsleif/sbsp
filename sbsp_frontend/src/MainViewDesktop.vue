<script setup lang="ts">
import RenumberDialog from './components/dialog/RenumberDialog.vue';
import AppFooter from './components/pc/AppFooter.vue';
import AppHeader from './components/pc/AppHeader.vue';
import BottomEditor from './components/pc/BottomEditor.vue';
import CueList from './components/pc/CueList.vue';
import { useUiState } from './stores/uiState.ts';
import { useShowModel } from './stores/showModel.ts';
import { useApi } from './api/index.ts';
import { storeToRefs } from 'pinia';
import { computed, ref, toRaw, watch } from 'vue';
import type { Cue } from './types/Cue.ts';
import { debounce } from './utils.ts';
import SideBar from './components/pc/SideBar.vue';
import ServerPanelDialog from './components/dialog/ServerPanelDialog.vue';
import SettingsDialog from './components/dialog/SettingsDialog.vue';
import FileListDialog from './components/dialog/FileListDialog.vue';
import BottomViewer from './components/pc/BottomViewer.vue';

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const uiState = useUiState();
const api = useApi();

const isHost = __IS_HOST__;

const selectedCue = ref<Cue | null>(
  uiState.selected != null ? getCueById.value(uiState.selected)! : null,
);
const selectedCueChainOverride = computed(() => {
  if (selectedCue.value == null) {
    return null;
  }
  const flatEntry = showModel.flatCueList.find((item) => item.cue.id === selectedCue.value!.id);
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

watch(
  () => showModel.cues,
  () => {
    selectedCue.value = uiState.selected != null ? getCueById.value(uiState.selected)! : null;
  },
);

const onCueEdited = debounce(() => {
  if (selectedCue.value == null) {
    return;
  }
  api.updateCue(toRaw(selectedCue.value));
}, 200);
</script>

<template>
  <div
    class="flex h-dvh w-screen flex-col"
    @contextmenu.prevent
  >
    <header class="h-50 shrink-0">
      <AppHeader />
    </header>
    <div class="flex w-full shrink grow flex-row overflow-hidden">
      <div class="flex h-full min-w-0 shrink grow flex-col">
        <main class="shrink grow overflow-hidden">
          <CueList />
        </main>
        <section
          class="shrink-0 grow-0 overflow-y-hidden border-t border-(--p-form-field-border-color) transition-[height]"
          :class="[uiState.isBottomTabOpen ? (uiState.mode === 'edit' ? 'h-62' : 'h-16') : 'h-0']"
        >
          <BottomEditor
            v-if="uiState.isBottomTabOpen && uiState.mode === 'edit'"
            v-model="selectedCue"
            :chain-override="selectedCueChainOverride"
            @update="onCueEdited"
          />
          <BottomViewer
            v-else-if="uiState.isBottomTabOpen"
            v-model="selectedCue"
          />
        </section>
      </div>
      <aside
        class="shrink-0 grow-0 overflow-hidden transition-[width]"
        :class="[uiState.isRightSidebarOpen ? 'w-65' : 'w-0']"
      >
        <SideBar />
      </aside>
    </div>

    <footer class="shrink-0">
      <AppFooter />
    </footer>

    <renumber-dialog v-model="uiState.isRenumberCueDialogOpen" />
    <settings-dialog v-model="uiState.isSettingsDialogOpen" />
    <file-list-dialog
      v-if="!isHost"
      v-model="uiState.fileListResolver"
      :multiple="uiState.fileListOption"
    />
    <server-panel-dialog
      v-if="isHost"
      v-model="uiState.isServerPanelOpen"
    />
  </div>
</template>
