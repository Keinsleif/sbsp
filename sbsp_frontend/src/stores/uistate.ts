import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';
import { getLockCursorToSelection } from '../utils';

export const useUiState = defineStore(
  'uistate',
  () => {
    const side = ref<'remote' | 'main' | null>(null);
    const selected = ref<string | null>(null);
    const selectedRows = ref<string[]>([]);
    const sideBarTab = ref<'activeCues' | 'levels'>('activeCues');
    const isRightSidebarOpen = ref(true);
    const isRenumberCueDialogOpen = ref(false);
    const isUpdateDialogOpen = ref(false);
    const isSettingsDialogOpen = ref(false);
    const isCreditsDialogOpen = ref(false);
    const isLicenseDialogOpen = ref(false);
    const isEditorOpen = ref(true);
    const success_messages = ref<string[]>([]);
    const error_messages = ref<string[]>([]);

    const clearSelected = () => {
      selected.value = null;
      selectedRows.value = [];
      if (getLockCursorToSelection()) {
        invoke('set_playback_cursor', {
          cueId: null,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };
    const setSelected = (id: string) => {
      selected.value = id;
      selectedRows.value = [id];
      if (getLockCursorToSelection()) {
        invoke('set_playback_cursor', {
          cueId: id,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };
    const addSelected = (id: string) => {
      selected.value = id;
      if (!selectedRows.value.includes(id)) {
        selectedRows.value.push(id);
      }
      if (getLockCursorToSelection()) {
        invoke('set_playback_cursor', {
          cueId: id,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };
    const toggleRightSidebar = () => {
      isRightSidebarOpen.value = !isRightSidebarOpen.value;
    };
    const toggleEditor = () => {
      isEditorOpen.value = !isEditorOpen.value;
    };
    const success = (message: string) => {
      success_messages.value.push(message);
    };
    const error = (message: string) => {
      error_messages.value.push(message);
    };

    return {
      side,
      selected,
      selectedRows,
      sideBarTab,
      isRightSidebarOpen,
      isRenumberCueDialogOpen,
      isSettingsDialogOpen,
      isUpdateDialogOpen,
      isCreditsDialogOpen,
      isLicenseDialogOpen,
      isEditorOpen,
      success_messages,
      error_messages,
      clearSelected,
      setSelected,
      addSelected,
      toggleRightSidebar,
      toggleEditor,
      success,
      error,
    };
  },
  {
    persist: {
      omit: ['side', 'selected', 'selectedRows', 'success_messages', 'error_messages'],
    },
  },
);
