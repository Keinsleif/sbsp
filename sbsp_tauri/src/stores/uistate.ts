import { defineStore } from 'pinia';

export const useUiState = defineStore('uistate', {
  state: () => ({
    selected: null as string | null,
    selectedRows: [] as string[],
    sideBarTab: 'activeCues' as 'activeCues' | 'levels',
    editorTab: 'basics',
    isRightSidebarOpen: true,
    isEditorOpen: true,
  }),
  actions: {
    toggleRightSidebar() {
      this.isRightSidebarOpen = !this.isRightSidebarOpen;
    },
    toggleEditor() {
      this.isEditorOpen = !this.isEditorOpen;
    },
  },
});
