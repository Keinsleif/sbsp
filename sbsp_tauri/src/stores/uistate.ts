import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { useShowModel } from './showmodel';

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
      const shwoModel = useShowModel();
      this.selected = null;
      this.selectedRows = [];
      if (shwoModel.settings.general.lockCursorToSelection) {
        invoke('set_playback_cursor', {
          cueId: null,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    },
    setSelected(id: string) {
      const shwoModel = useShowModel();
      this.selected = id;
      this.selectedRows = [id];
      if (shwoModel.settings.general.lockCursorToSelection) {
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
