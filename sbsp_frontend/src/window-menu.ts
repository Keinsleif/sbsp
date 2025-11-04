import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';

const side = await invoke<string>('get_side', {});

const fileMenu = await Submenu.new({
  text: 'File',
  items: [
    await MenuItem.new({
      id: 'id_open',
      text: 'Open',
      enabled: side == 'main',
      action: () => {
        invoke('file_open', {}).catch((e) => console.error(e));
      },
    }),
    await MenuItem.new({
      id: 'id_save',
      text: 'Save',
      enabled: side == 'main',
      action: () => {
        invoke('file_save', {}).catch((e) => console.error(e));
      },
    }),
    await MenuItem.new({
      id: 'id_save_as',
      text: 'Save As...',
      enabled: side == 'main',
      action: () => {
        invoke('file_save_as', {}).catch((e) => console.error(e));
      },
    }),
    await MenuItem.new({
      id: 'id_export_to_folder',
      text: 'Export to Folder',
      enabled: side == 'main',
      action: () => {
        invoke('export_to_folder', {}).catch((e) => console.error(e));
      },
    }),
  ],
});

const editMenu = await Submenu.new({
  text: 'Edit',
  items: [
    await PredefinedMenuItem.new({
      item: 'Cut',
    }),
    await PredefinedMenuItem.new({
      item: 'Copy',
    }),
    await PredefinedMenuItem.new({
      item: 'Paste',
    }),
    await MenuItem.new({
      id: 'id_delete',
      text: 'Delete',
      action: () => {
        const uiState = useUiState();
        for (const row of uiState.selectedRows) {
          invoke('remove_cue', { cueId: row }).catch((e) => console.error(e));
        }
      },
    }),
    await PredefinedMenuItem.new({
      item: 'SelectAll',
    }),
  ],
});

const cueMenu = await Submenu.new({
  text: 'Cue',
  items: [
    await MenuItem.new({
      id: 'id_audio_cue',
      text: 'Audio Cue',
      action: () => {
        const uiState = useUiState();
        const showModel = useShowModel();
        let insertIndex;
        if (uiState.selected) {
          insertIndex = showModel.cues.findIndex((cue) => cue.id == uiState.selected) + 1;
        } else {
          insertIndex = showModel.cues.length;
        }
        invoke('add_empty_cue', { cueType: 'audio', atIndex: insertIndex }).catch((e) => console.error(e));
      },
    }),
    await MenuItem.new({
      id: 'id_wait_cue',
      text: 'Wait Cue',
      action: () => {
        const uiState = useUiState();
        const showModel = useShowModel();
        let insertIndex;
        if (uiState.selected) {
          insertIndex = showModel.cues.findIndex((cue) => cue.id == uiState.selected) + 1;
        } else {
          insertIndex = showModel.cues.length;
        }
        invoke('add_empty_cue', { cueType: 'wait', atIndex: insertIndex }).catch((e) => console.error(e));
      },
    }),
  ],
});

const toolsMenu = await Submenu.new({
  text: 'Tools',
  items: [
    await MenuItem.new({
      id: 'id_renumber',
      text: 'Renumber selected cues',
      action: () => {
        const uiState = useUiState();
        uiState.isRenumberCueDialogOpen = true;
      },
    }),
  ],
});

const helpMenu = await Submenu.new({
  text: 'Help',
  items: [
    await MenuItem.new({
      id: 'id_check_update',
      text: 'Check for updates',
      action: () => {
        const uiState = useUiState();
        uiState.isUpdateDialogOpen = true;
      },
    }),
  ],
});

export const menu = await Menu.new({
  items: [fileMenu, editMenu, cueMenu, toolsMenu, helpMenu],
});
