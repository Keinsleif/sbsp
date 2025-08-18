import { defineStore } from 'pinia';
import { useUiSettings } from './uisettings';
import { invoke } from '@tauri-apps/api/core';

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
    clearSelected() {
      const uiSettings = useUiSettings();
      this.selected = null;
      this.selectedRows = [];
      if (uiSettings.lockCursorToSelection) {
        invoke('set_playback_cursor', {
          cueId: null,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    },
    setSelected(id: string) {
      const uiSettings = useUiSettings();
      this.selected = id;
      this.selectedRows = [id];
      if (uiSettings.lockCursorToSelection) {
        invoke('set_playback_cursor', {
          cueId: id,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    },
    toggleRightSidebar() {
      this.isRightSidebarOpen = !this.isRightSidebarOpen;
    },
    toggleEditor() {
      this.isEditorOpen = !this.isEditorOpen;
    },
  },
});
