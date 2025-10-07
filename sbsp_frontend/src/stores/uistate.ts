import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { useShowModel } from './showmodel';
import { ref } from 'vue';

export const useUiState = defineStore(
  'uistate',
  () => {
    const side = ref<'remote' | 'main' | null>(null);
    const selected = ref<string | null>(null);
    const selectedRows = ref<string[]>([]);
    const sideBarTab = ref<'activeCues' | 'levels'>('activeCues');
    const isRightSidebarOpen = ref(true);
    const isSettingsDialogOpen = ref(false);
    const isEditorOpen = ref(true);
    const success_messages = ref<string[]>([]);
    const error_messages = ref<string[]>([]);

    const clearSelected = () => {
      const shwoModel = useShowModel();
      selected.value = null;
      selectedRows.value = [];
      if (shwoModel.getLockCursorToSelection()) {
        invoke('set_playback_cursor', {
          cueId: null,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };
    const setSelected = (id: string) => {
      const showModel = useShowModel();
      selected.value = id;
      selectedRows.value = [id];
      if (showModel.getLockCursorToSelection()) {
        invoke('set_playback_cursor', {
          cueId: id,
        }).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };
    const addSelected = (id: string) => {
      const showModel = useShowModel();
      selected.value = id;
      if (!selectedRows.value.includes(id)) {
        selectedRows.value.push(id);
      }
      if (showModel.getLockCursorToSelection()) {
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
      isSettingsDialogOpen,
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
