import { defineStore } from 'pinia';
import { ref } from 'vue';
import { getLockCursorToSelection } from '../utils';
import { useApi } from '../api';
import { useShowModel } from './showmodel';

export const useUiState = defineStore(
  'uistate',
  () => {
    const mode = ref<'edit' | 'run' | 'view'>('edit');
    const selected = ref<string | null>(null);
    const selectedRows = ref<string[]>([]);
    const expandedRows = ref<string[]>([]);
    const preWaitDisplayMode = ref<'elapsed' | 'remain'>('elapsed');
    const durationDisplayMode = ref<'elapsed' | 'remain'>('elapsed');
    const postWaitDisplayMode = ref<'elapsed' | 'remain'>('elapsed');
    const sideBarTab = ref<'activeCues' | 'levels'>('activeCues');
    const isRightSidebarOpen = ref(true);
    const isRenumberCueDialogOpen = ref(false);
    const isUpdateDialogOpen = ref(false);
    const isSettingsDialogOpen = ref(false);
    const isCreditsDialogOpen = ref(false);
    const isLicenseDialogOpen = ref(false);
    const isServerPanelOpen = ref(false);
    const fileListResolver = ref<((select: string[] | null) => void) | null>(null);
    const fileListOption = ref(false);
    const isBottomTabOpen = ref(true);
    const scaleWaveform = ref(true);
    const success_messages = ref<string[]>([]);
    const error_messages = ref<string[]>([]);

    const setPlaybackCursor = (id: string | null) => {
      const api = useApi();
      if (getLockCursorToSelection()) {
        api.setPlaybackCursor(id).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };

    const clearSelected = () => {
      selected.value = null;
      selectedRows.value = [];
      setPlaybackCursor(null);
    };
    const setSelected = (id: string) => {
      selected.value = id;
      selectedRows.value = [id];
      setPlaybackCursor(id);
    };
    const addSelected = (id: string) => {
      selected.value = id;
      if (!selectedRows.value.includes(id)) {
        selectedRows.value.push(id);
      }
      setPlaybackCursor(id);
    };
    const removeFromSelected = (id: string) => {
      if (selectedRows.value.includes(id)) {
        selectedRows.value = selectedRows.value.filter((selected) => selected != id);
        const newValue = selectedRows.value[selectedRows.value.length - 1] || null;
        selected.value = newValue;
        setPlaybackCursor(selected.value);
      }
    };

    const toggleExpand = (id: string) => {
      if (expandedRows.value.includes(id)) {
        expandedRows.value.splice(
          expandedRows.value.findIndex((value) => value == id),
          1,
        );
      } else {
        expandedRows.value.push(id);
      }
    };

    const expandToVisible = (id: string) => {
      const showModel = useShowModel();
      let target_id: string | null = id;
      while (target_id != null) {
        const target_cue = showModel.flatCueList.find((value) => value.cue.id == target_id);
        if (target_cue != null && target_cue.parent != null) {
          if (!expandedRows.value.includes(target_cue.parent)) {
            expandedRows.value.push(target_cue.parent);
          }
          target_id = target_cue.parent;
        } else {
          target_id = null;
        }
      }
    };

    const togglePreWaitDisplayMode = () => {
      preWaitDisplayMode.value = preWaitDisplayMode.value == 'elapsed' ? 'remain' : 'elapsed';
    };

    const toggleDurationDisplayMode = () => {
      durationDisplayMode.value = durationDisplayMode.value == 'elapsed' ? 'remain' : 'elapsed';
    };

    const togglePostWaitDisplayMode = () => {
      postWaitDisplayMode.value = postWaitDisplayMode.value == 'elapsed' ? 'remain' : 'elapsed';
    };

    const toggleRightSidebar = () => {
      isRightSidebarOpen.value = !isRightSidebarOpen.value;
    };
    const toggleBottomTab = () => {
      isBottomTabOpen.value = !isBottomTabOpen.value;
    };
    const success = (message: string) => {
      success_messages.value.push(message);
    };
    const error = (message: string) => {
      error_messages.value.push(message);
    };

    return {
      mode,
      selected,
      selectedRows,
      expandedRows,
      preWaitDisplayMode,
      durationDisplayMode,
      postWaitDisplayMode,
      sideBarTab,
      isRightSidebarOpen,
      isRenumberCueDialogOpen,
      isSettingsDialogOpen,
      isUpdateDialogOpen,
      isCreditsDialogOpen,
      isLicenseDialogOpen,
      isServerPanelOpen,
      fileListResolver,
      fileListOption,
      isBottomTabOpen,
      scaleWaveform,
      success_messages,
      error_messages,
      setPlaybackCursor,
      clearSelected,
      setSelected,
      addSelected,
      removeFromSelected,
      toggleExpand,
      expandToVisible,
      togglePreWaitDisplayMode,
      toggleDurationDisplayMode,
      togglePostWaitDisplayMode,
      toggleRightSidebar,
      toggleBottomTab,
      success,
      error,
    };
  },
  {
    persist: {
      pick: [
        'mode',
        'preWaitDisplayMode',
        'durationDisplayMode',
        'postWaitDisplayMode',
        'sideBarTab',
        'isRightSidebarOpen',
        'isBottomTabOpen',
        'scaleWaveform',
      ],
    },
  },
);
