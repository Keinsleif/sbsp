<script setup lang="ts">
import AppFooter from './components/pc/AppFooter.vue';
import AppHeader from './components/pc/AppHeader.vue';
import { useUiState } from './stores/uiState.ts';

const uiState = useUiState();
</script>

<template>
  <div class="flex flex-col h-dvh w-screen" @contextmenu.prevent>
    <header class="h-50">
      <AppHeader />
    </header>
    <div class="flex flex-row grow w-full">
      <div class="flex flex-col grow">
        <main class="grow">main area</main>
        <section
          class="transition-[height] overflow-hidden"
          :class="[uiState.isBottomTabOpen ? 'h-62.5' : 'h-0']"
        >
          editor
        </section>
      </div>
      <aside
        class="transition-[width]"
        :class="[uiState.isRightSidebarOpen ? 'w-65' : 'w-0']"
      >
        sidebar
      </aside>
    </div>

    <footer>
      <AppFooter />
    </footer>
  </div>
</template>