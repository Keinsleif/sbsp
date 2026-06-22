// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { defineStore } from 'pinia';
import { ref } from 'vue';
import { getLockCursorToSelection, PERMISSIONS } from '../utils.ts';
import { useApi } from '../api';
import { useShowModel } from './showModel';
import type { Permissions } from '../types/Permissions';
import { useToast } from 'primevue/usetoast';
import type { ToastServiceMethods } from 'primevue/toastservice';

export const useUiState = defineStore(
  'uiState',
  () => {
    const permission = ref<Permissions>(0b0111);
    const mode = ref<'edit' | 'run' | 'view'>('edit');
    const selected = ref<string | null>(null);
    const selectedRows = ref<Set<string>>(new Set());
    const expandedRows = ref<string[]>([]);
    const preWaitDisplayMode = ref<'elapsed' | 'remain'>('elapsed');
    const durationDisplayMode = ref<'elapsed' | 'remain'>('elapsed');
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
    const isEnvelopeVisible = ref(false);
    const scaleWaveform = ref(true);
    const lastUpdateCheckDate = ref<number>(0);
    let toast: ToastServiceMethods | null = null;

    const setPlaybackCursor = (id: string | null) => {
      const api = useApi();
      if (getLockCursorToSelection()) {
        api.setPlaybackCursor(id).catch((e) => {
          console.error('Failed to set cursor. ' + e);
        });
      }
    };

    const resetSelected = () => {
      selected.value = null;
      selectedRows.value.clear();
    };
    const clearSelected = () => {
      resetSelected();
      setPlaybackCursor(null);
    };
    const setSelected = (id: string) => {
      selected.value = id;
      selectedRows.value.clear();
      selectedRows.value.add(id);
      setPlaybackCursor(id);
    };
    const addSelected = (id: string) => {
      selected.value = id;
      selectedRows.value.add(id);
      setPlaybackCursor(id);
    };
    const removeFromSelected = (ids: string[]) => {
      let rm_selected = false;
      for (const id of ids) {
        selectedRows.value.delete(id);
        if (!rm_selected && id === selected.value) {
          rm_selected = true;
        }
      }
      if (rm_selected) {
        const newValue = selectedRows.value.values().next().value || null;
        selected.value = newValue;
        setPlaybackCursor(selected.value);
      }
    };

    const toggleExpand = (id: string) => {
      if (expandedRows.value.includes(id)) {
        expandedRows.value.splice(
          expandedRows.value.findIndex((value) => value === id),
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
        const target_cue = showModel.flatCueList.find((value) => value.cue.id === target_id);
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
      preWaitDisplayMode.value = preWaitDisplayMode.value === 'elapsed' ? 'remain' : 'elapsed';
    };

    const toggleDurationDisplayMode = () => {
      durationDisplayMode.value = durationDisplayMode.value === 'elapsed' ? 'remain' : 'elapsed';
    };

    const toggleRightSidebar = () => {
      isRightSidebarOpen.value = !isRightSidebarOpen.value;
    };
    const toggleBottomTab = () => {
      isBottomTabOpen.value = !isBottomTabOpen.value;
    };
    const success = (title: string, detail?: string, timeout?: number) => {
      if (toast == null) {
        toast = useToast();
      }
      toast.add({ severity: 'success', summary: title, detail, life: timeout || 2000 });
    };
    const error = (title: string, detail?: string, timeout?: number) => {
      if (toast == null) {
        toast = useToast();
      }
      toast.add({ severity: 'error', summary: title, detail, life: timeout || 3000 });
    };

    const setPermission = (perm: Permissions) => {
      permission.value = perm;
      validateMode();
    };

    const validateMode = () => {
      if (permission.value == null) return;
      if (!(permission.value & modeAsPerm())) {
        const highestBitPos = 31 - Math.clz32(permission.value);
        switch (highestBitPos) {
          case 0:
            mode.value = 'view';
            break;
          case 1:
            mode.value = 'run';
            break;
          case 2:
            mode.value = 'edit';
            break;
        }
      }
    };

    const modeAsPerm = () => {
      switch (mode.value) {
        case 'edit':
          return PERMISSIONS.EDIT;
        case 'run':
          return PERMISSIONS.CONTROL;
        case 'view':
          return PERMISSIONS.READ;
      }
    };

    return {
      permission,
      mode,
      selected,
      selectedRows,
      expandedRows,
      preWaitDisplayMode,
      durationDisplayMode,
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
      isEnvelopeVisible,
      scaleWaveform,
      lastUpdateCheckDate,
      setPermission,
      setPlaybackCursor,
      resetSelected,
      clearSelected,
      setSelected,
      addSelected,
      removeFromSelected,
      toggleExpand,
      expandToVisible,
      togglePreWaitDisplayMode,
      toggleDurationDisplayMode,
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
        'sideBarTab',
        'isRightSidebarOpen',
        'isBottomTabOpen',
        'scaleWaveform',
        'lastUpdateCheckDate',
      ],
    },
  },
);
