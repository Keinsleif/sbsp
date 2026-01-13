import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';
import { i18n } from './i18n';
import { platform } from '@tauri-apps/plugin-os';
import { message } from '@tauri-apps/plugin-dialog';
import { useUiSettings } from './stores/uiSettings';
import { useApi } from './api';

export const createWindowMenu = async () => {
  const api = useApi();
  if (api.target != 'tauri') return;
  const side = api.side;
  const { t } = i18n.global;
  const currentPlatform = platform();

  let remoteFileMenuItem: (PredefinedMenuItem | MenuItem)[] = [];
  if (side == 'remote') {
    remoteFileMenuItem = [
      await PredefinedMenuItem.new({
        item: 'Separator',
      }),
      await MenuItem.new({
        id: 'id_disconnect',
        text: t('menu.file.disconnect'),
        action: () => {
          api.remote?.disconnectFromServer();
        },
      }),
    ];
  }

  const fileMenu = await Submenu.new({
    text: t('menu.file.title'),
    items: [
      await MenuItem.new({
        id: 'id_new',
        text: t('menu.file.new'),
        enabled: side == 'host',
        action: () => {
          api.isModified().then((isModified) => {
            if (isModified) {
              message(t('dialog.saveConfirm.content'), {
                buttons: {
                  yes: t('dialog.saveConfirm.save'),
                  no: t('dialog.saveConfirm.dontSave'),
                  cancel: t('dialog.saveConfirm.cancel'),
                },
              })
                .then((result) => {
                  switch (result) {
                    case t('dialog.saveConfirm.save'):
                      api.host
                        ?.fileSave()
                        .then((isSaved) => {
                          if (isSaved) {
                            api.host?.fileNew();
                          }
                        })
                        .catch((e) => console.error(e));
                      break;
                    case t('dialog.saveConfirm.dontSave'):
                      api.host?.fileNew();
                      break;
                    case t('dialog.saveConfirm.cancel'):
                      break;
                  }
                })
                .catch((e) => console.error(e));
            } else {
              api.host?.fileNew();
            }
          });
        },
      }),
      await MenuItem.new({
        id: 'id_open',
        text: t('menu.file.open'),
        enabled: side == 'host',
        accelerator: currentPlatform == 'macos' ? '⌘ + O' : 'Ctrl + O',
        action: () => {
          api.host?.fileOpen();
        },
      }),
      await MenuItem.new({
        id: 'id_save',
        text: t('menu.file.save'),
        enabled: side == 'host',
        accelerator: currentPlatform == 'macos' ? '⌘ + S' : 'Ctrl + S',
        action: () => {
          api.host?.fileSave();
        },
      }),
      await MenuItem.new({
        id: 'id_save_as',
        text: t('menu.file.saveAs'),
        enabled: side == 'host',
        accelerator: currentPlatform == 'macos' ? '⇧ + ⌘ + S' : 'Ctrl + Shift + S',
        action: () => {
          api.host?.fileSaveAs();
        },
      }),
      await MenuItem.new({
        id: 'id_export_to_folder',
        text: t('menu.file.exportToFolder'),
        enabled: side == 'host',
        action: () => {
          api.host?.exportToFolder();
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
        // enabled: uiState.mode == 'edit',
        accelerator: currentPlatform == 'macos' ? '⌘ + ⌫' : 'Ctrl + Backspace',
        action: () => {
          const uiState = useUiState();
          for (const row of uiState.selectedRows) {
            api.removeCue(row);
          }
        },
      }),
      await PredefinedMenuItem.new({
        item: 'SelectAll',
        text: t('menu.edit.selectAll'),
      }),
      await PredefinedMenuItem.new({
        item: 'Separator',
      }),
      await MenuItem.new({
        id: 'id_import_settings',
        text: t('menu.edit.importSettings'),
        action: () => {
          const uiSettings = useUiSettings();
          uiSettings.import_from_file();
        },
      }),
      await MenuItem.new({
        id: 'id_export_settings',
        text: t('menu.edit.exportSettings'),
        action: () => {
          const uiSettings = useUiSettings();
          uiSettings.export_to_file();
        },
      }),
    ],
  });

  const cueMenu = await Submenu.new({
    text: t('menu.cue.title'),
    items: [
      await MenuItem.new({
        id: 'id_audio_cue',
        text: t('menu.cue.audio'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyAudioCue();
        },
      }),
      await MenuItem.new({
        id: 'id_wait_cue',
        text: t('menu.cue.wait'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyWaitCue();
        },
      }),
      await MenuItem.new({
        id: 'id_fade_cue',
        text: t('menu.cue.fade'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyFadeCue();
        },
      }),
      await MenuItem.new({
        id: 'id_start_cue',
        text: t('menu.cue.start'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('start');
        },
      }),
      await MenuItem.new({
        id: 'id_stop_cue',
        text: t('menu.cue.stop'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('stop');
        },
      }),
      await MenuItem.new({
        id: 'id_pause_cue',
        text: t('menu.cue.pause'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('pause');
        },
      }),
      await MenuItem.new({
        id: 'id_load_cue',
        text: t('menu.cue.load'),
        // enabled: uiState.mode == 'edit',
        action: () => {
          const showModel = useShowModel();
          showModel.addEmptyPlaybackCue('load');
        },
      }),
      await MenuItem.new({
        id: 'id_group_cue',
        text: t('menu.cue.group'),
        // enabled: uiState.mode == 'edit',
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
        // enabled: uiState.mode == 'edit',
        action: () => {
          const uiState = useUiState();
          uiState.isRenumberCueDialogOpen = true;
        },
      }),
    ],
  });

  let mainHelpMenu: MenuItem[] = [];
  if (side == 'host') {
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
