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

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const uiState = useUiState();
const api = useApi();

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
    class="flex flex-col h-dvh w-screen"
    @contextmenu.prevent
  >
    <header class="h-50 shrink-0">
      <AppHeader />
    </header>
    <div class="flex flex-row grow w-full overflow-hidden">
      <div class="flex flex-col h-full grow">
        <main class="grow shrink base-0 overflow-hidden">
          <CueList />
        </main>
        <section
          class="transition-[height] overflow-hidden shrink-0 grow-0"
          :class="[uiState.isBottomTabOpen ? 'h-62.5' : 'h-0']"
        >
          <BottomEditor
            v-model="selectedCue"
            :chain-override="selectedCueChainOverride"
            @update="onCueEdited"
          />
        </section>
      </div>
      <aside
        class="transition-[width] shrink-0 grow-0 overflow-hidden"
        :class="[uiState.isRightSidebarOpen ? 'w-65' : 'w-0']"
      >
        <SideBar />
      </aside>
    </div>

    <footer class="shrink-0">
      <AppFooter />
    </footer>

    <renumber-dialog v-model="uiState.isRenumberCueDialogOpen" />
  </div>
</template>
