import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { useShowModel } from './showmodel';

export const useUiState = defineStore('uistate', {
  state: () => ({
    side: null as string | null,
    selected: null as string | null,
    selectedRows: [] as string[],
    sideBarTab: 'activeCues' as 'activeCues' | 'levels',
    isRightSidebarOpen: true,
    isSettingsDialogOpen: false,
    isEditorOpen: true,
    success_messages: [] as string[],
    error_messages: [] as string[],
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
      const showModel = useShowModel();
      this.selected = id;
      this.selectedRows = [id];
      if (showModel.settings.general.lockCursorToSelection) {
        invoke('set_playback_cursor', {
          cueId: id,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    },
    addSelected(id: string) {
      const showModel = useShowModel();
      this.selected = id;
      if (!this.selectedRows.includes(id)) {
        this.selectedRows.push(id);
      }
      if (showModel.settings.general.lockCursorToSelection) {
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
    success(message: string) {
      this.success_messages.push(message);
    },
    error(message: string) {
      this.error_messages.push(message);
    },
  },
});
