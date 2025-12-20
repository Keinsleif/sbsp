import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';
import { i18n } from './i18n';
import { platform } from '@tauri-apps/plugin-os';

export const createWindowMenu = async () => {
  const { t } = i18n.global;
  const uiState = useUiState();
  const currentPlatform = platform();

  let remoteFileMenuItem: (PredefinedMenuItem | MenuItem)[] = [];
  if (uiState.side == 'remote') {
    remoteFileMenuItem = [
      await PredefinedMenuItem.new({
        item: 'Separator',
      }),
      await MenuItem.new({
        id: 'id_disconnect',
        text: t('menu.file.disconnect'),
        action: () => {
          invoke('disconnect_from_server', {}).catch((e) => console.error(e));
        },
      }),
    ];
  }

  const fileMenu = await Submenu.new({
    text: t('menu.file.title'),
    items: [
      await MenuItem.new({
        id: 'id_open',
        text: t('menu.file.open'),
        enabled: uiState.side == 'main',
        action: () => {
          invoke('file_open', {}).catch((e) => console.error(e));
        },
      }),
      await MenuItem.new({
        id: 'id_save',
        text: t('menu.file.save'),
        enabled: uiState.side == 'main',
        action: () => {
          invoke('file_save', {}).catch((e) => console.error(e));
        },
      }),
      await MenuItem.new({
        id: 'id_save_as',
        text: t('menu.file.saveAs'),
        enabled: uiState.side == 'main',
        action: () => {
          invoke('file_save_as', {}).catch((e) => console.error(e));
        },
      }),
      await MenuItem.new({
        id: 'id_export_to_folder',
        text: t('menu.file.exportToFolder'),
        enabled: uiState.side == 'main',
        action: () => {
          invoke('export_to_folder', {}).catch((e) => console.error(e));
        },
      }),
      ...remoteFileMenuItem,
    ],
  });

  const editMenu = await Submenu.new({
    text: t('menu.edit.title'),
    items: [
      await PredefinedMenuItem.new({
        item: 'Cut',
        text: t('menu.edit.cut'),
      }),
      await PredefinedMenuItem.new({
        item: 'Copy',
        text: t('menu.edit.copy'),
      }),
      await PredefinedMenuItem.new({
        item: 'Paste',
        text: t('menu.edit.paste'),
      }),
      await MenuItem.new({
        id: 'id_delete',
        text: t('menu.edit.delete'),
        enabled: uiState.mode == 'edit',
        accelerator: currentPlatform == 'macos' ? '⌘ + Delete' : 'Ctrl + Backspace',
        action: () => {
          const uiState = useUiState();
          for (const row of uiState.selectedRows) {
            invoke('remove_cue', { cueId: row }).catch((e) => console.error(e));
          }
        },
      }),
      await PredefinedMenuItem.new({
        item: 'SelectAll',
        text: t('menu.edit.selectAll'),
      }),
    ],
  });

  const cueMenu = await Submenu.new({
    text: t('menu.cue.title'),
    items: [
      await MenuItem.new({
        id: 'id_audio_cue',
        text: t('menu.cue.audio'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyAudioCue();
        },
      }),
      await MenuItem.new({
        id: 'id_wait_cue',
        text: t('menu.cue.wait'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyWaitCue();
        },
      }),
      await MenuItem.new({
        id: 'id_fade_cue',
        text: t('menu.cue.fade'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyFadeCue();
        },
      }),
      await MenuItem.new({
        id: 'id_start_cue',
        text: t('menu.cue.start'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('start');
        },
      }),
      await MenuItem.new({
        id: 'id_stop_cue',
        text: t('menu.cue.stop'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('stop');
        },
      }),
      await MenuItem.new({
        id: 'id_pause_cue',
        text: t('menu.cue.pause'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('pause');
        },
      }),
      await MenuItem.new({
        id: 'id_load_cue',
        text: t('menu.cue.load'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('load');
        },
      }),
      await MenuItem.new({
        id: 'id_group_cue',
        text: t('menu.cue.group'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyGroupCue();
        },
      }),
    ],
  });

  const toolsMenu = await Submenu.new({
    text: t('menu.tools.title'),
    items: [
      await MenuItem.new({
        id: 'id_renumber',
        text: t('menu.tools.renumber'),
        enabled: uiState.mode == 'edit',
        action: () => {
          const uiState = useUiState();
          uiState.isRenumberCueDialogOpen = true;
        },
      }),
    ],
  });

  let mainHelpMenu: MenuItem[] = [];
  if (uiState.side == 'main') {
    mainHelpMenu = [
      await MenuItem.new({
        id: 'id_license',
        text: t('menu.help.license'),
        action: () => {
          const uiState = useUiState();
          uiState.isLicenseDialogOpen = true;
        },
      }),
    ];
  }

  const helpMenu = await Submenu.new({
    text: t('menu.help.title'),
    items: [
      await MenuItem.new({
        id: 'id_credits',
        text: t('menu.help.credits'),
        action: () => {
          const uiState = useUiState();
          uiState.isCreditsDialogOpen = true;
        },
      }),
      await MenuItem.new({
        id: 'id_check_update',
        text: t('menu.help.checkUpdate'),
        action: () => {
          const uiState = useUiState();
          uiState.isUpdateDialogOpen = true;
        },
      }),
      ...mainHelpMenu,
    ],
  });

  return await Menu.new({
    items: [fileMenu, editMenu, cueMenu, toolsMenu, helpMenu],
  });
};
