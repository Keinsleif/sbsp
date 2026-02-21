import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { useUiState } from './stores/uistate';
import { useShowModel } from './stores/showmodel';
import { i18n } from './i18n';
import { message } from '@tauri-apps/plugin-dialog';
import { useUiSettings } from './stores/uiSettings';
import { useApi, side, target } from './api';
import { appLogDir } from '@tauri-apps/api/path';
import { openPath } from '@tauri-apps/plugin-opener';

type MenuItemHolder = MenuItem | PredefinedMenuItem | null;

export const createWindowMenu = () => {
  const api = useApi();
  if (target != 'tauri') return;
  const { t } = i18n.global;
  const isMacOs = api.isMacOs();
  let connected = api.remote ? false : true;
  const uiState = useUiState();
  let mode: 'edit' | 'run' | 'view' = uiState.mode;

  const items = {
    file: {
      new: null as MenuItemHolder,
      open: null as MenuItemHolder,
      save: null as MenuItemHolder,
      saveAs: null as MenuItemHolder,
      exportToFolder: null as MenuItemHolder,
      disconnect: null as MenuItemHolder,
    },
    edit: {
      cut: null as MenuItemHolder,
      copy: null as MenuItemHolder,
      paste: null as MenuItemHolder,
      deleteCue: null as MenuItemHolder,
      selectAllCues: null as MenuItemHolder,
      importSettings: null as MenuItemHolder,
      exportSettings: null as MenuItemHolder,
    },
    cue: {
      audio: null as MenuItemHolder,
      wait: null as MenuItemHolder,
      fade: null as MenuItemHolder,
      start: null as MenuItemHolder,
      stop: null as MenuItemHolder,
      pause: null as MenuItemHolder,
      load: null as MenuItemHolder,
      group: null as MenuItemHolder,
    },
    tools: {
      renumber: null as MenuItemHolder,
    },
    help: {
      credits: null as MenuItemHolder,
      checkUpdate: null as MenuItemHolder,
      license: null as MenuItemHolder,
      showLogFiles: null as MenuItemHolder,
    },
  };

  const submenues: { [key in keyof typeof items]: Submenu | null } = {
    file: null,
    edit: null,
    cue: null,
    tools: null,
    help: null,
  };

  let menu: Menu | null = null;

  const updateLocale = () => {
    Object.entries(items).forEach(([submenuId, menus]) => {
      Object.entries(menus).forEach(([menuId, menuItem]) => {
        menuItem?.setText(t(`menu.${submenuId}.${menuId}`));
      });
      submenues[submenuId as keyof typeof items]?.setText(t(`menu.${submenuId}.title`));
    });
  };

  const updateConnectionStatus = (isConnected: boolean) => {
    if (side == 'remote') {
      connected = isConnected;
      (items.file.disconnect as MenuItem | null)?.setEnabled(connected);
      updateEditMenuItemStats();
    }
  };

  const updateEditMode = (newMode: 'edit' | 'run' | 'view') => {
    mode = newMode;
    updateEditMenuItemStats();
  };

  let lastEditEnableStats = connected && mode == 'edit';
  const updateEditMenuItemStats = () => {
    const enabled = connected && mode == 'edit';
    if (lastEditEnableStats == enabled) return;
    lastEditEnableStats = enabled;

    (items.edit.deleteCue as MenuItem | null)?.setEnabled(enabled);
    (items.edit.selectAllCues as MenuItem | null)?.setEnabled(enabled);
    Object.values(items.cue).forEach((value) => {
      (value as MenuItem | null)?.setEnabled(enabled);
    });
    (items.tools.renumber as MenuItem | null)?.setEnabled(enabled);
  };

  const init = async () => {
    let remoteFileMenuItem: (PredefinedMenuItem | MenuItem)[] = [];
    if (side == 'remote') {
      items.file.disconnect = await MenuItem.new({
        id: 'id_disconnect',
        text: t('menu.file.disconnect'),
        enabled: connected,
        action: () => {
          api.remote?.disconnectFromServer();
        },
      });
      remoteFileMenuItem = [
        await PredefinedMenuItem.new({
          item: 'Separator',
        }),
        items.file.disconnect,
      ];
    }

    items.file.new = await MenuItem.new({
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
    });

    items.file.open = await MenuItem.new({
      id: 'id_open',
      text: t('menu.file.open'),
      enabled: side == 'host',
      accelerator: isMacOs ? '⌘ + O' : 'Ctrl + O',
      action: () => {
        api.host?.fileOpen();
      },
    });

    items.file.save = await MenuItem.new({
      id: 'id_save',
      text: t('menu.file.save'),
      enabled: side == 'host',
      accelerator: isMacOs ? '⌘ + S' : 'Ctrl + S',
      action: () => {
        api.host?.fileSave();
      },
    });

    items.file.saveAs = await MenuItem.new({
      id: 'id_save_as',
      text: t('menu.file.saveAs'),
      enabled: side == 'host',
      accelerator: isMacOs ? '⇧ + ⌘ + S' : 'Ctrl + Shift + S',
      action: () => {
        api.host?.fileSaveAs();
      },
    });

    items.file.exportToFolder = await MenuItem.new({
      id: 'id_export_to_folder',
      text: t('menu.file.exportToFolder'),
      enabled: side == 'host',
      action: () => {
        api.host?.exportToFolder();
      },
    });

    submenues.file = await Submenu.new({
      text: t('menu.file.title'),
      items: [
        items.file.new,
        items.file.open,
        items.file.save,
        items.file.saveAs,
        items.file.exportToFolder,
        ...remoteFileMenuItem,
      ],
    });

    items.edit.cut = await PredefinedMenuItem.new({
      item: 'Cut',
      text: t('menu.edit.cut'),
    });

    items.edit.copy = await PredefinedMenuItem.new({
      item: 'Copy',
      text: t('menu.edit.copy'),
    });

    items.edit.paste = await PredefinedMenuItem.new({
      item: 'Paste',
      text: t('menu.edit.paste'),
    });

    items.edit.deleteCue = await MenuItem.new({
      id: 'id_delete',
      text: t('menu.edit.deleteCue'),
      enabled: lastEditEnableStats,
      accelerator: isMacOs ? '⌘ + ⌫' : 'Ctrl + Backspace',
      action: () => {
        const uiState = useUiState();
        for (const row of uiState.selectedRows) {
          api.removeCue(row);
        }
      },
    });

    items.edit.selectAllCues = await MenuItem.new({
      id: 'id_select_all_cues',
      text: t('menu.edit.selectAllCues'),
      enabled: lastEditEnableStats,
      accelerator: isMacOs ? '⌘ + ⌫' : 'Ctrl + Backspace',
      action: () => {
        const uiState = useUiState();
        const showModel = useShowModel();
        uiState.selectedRows = showModel.flatCueList.filter((item) => !item.isHidden).map((item) => item.cue.id);
      },
    });

    items.edit.importSettings = await MenuItem.new({
      id: 'id_import_settings',
      text: t('menu.edit.importSettings'),
      action: () => {
        const uiSettings = useUiSettings();
        uiSettings.import_from_file();
      },
    });

    items.edit.exportSettings = await MenuItem.new({
      id: 'id_export_settings',
      text: t('menu.edit.exportSettings'),
      action: () => {
        const uiSettings = useUiSettings();
        uiSettings.export_to_file();
      },
    });

    submenues.edit = await Submenu.new({
      text: t('menu.edit.title'),
      items: [
        items.edit.cut,
        items.edit.copy,
        items.edit.paste,
        items.edit.deleteCue,
        items.edit.selectAllCues,
        await PredefinedMenuItem.new({
          item: 'Separator',
        }),
        items.edit.importSettings,
        items.edit.exportSettings,
      ],
    });

    items.cue.audio = await MenuItem.new({
      id: 'id_audio_cue',
      text: t('menu.cue.audio'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyAudioCue();
      },
    });

    items.cue.wait = await MenuItem.new({
      id: 'id_wait_cue',
      text: t('menu.cue.wait'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyWaitCue();
      },
    });

    items.cue.fade = await MenuItem.new({
      id: 'id_fade_cue',
      text: t('menu.cue.fade'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyFadeCue();
      },
    });

    items.cue.start = await MenuItem.new({
      id: 'id_start_cue',
      text: t('menu.cue.start'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyPlaybackCue('start');
      },
    });

    items.cue.stop = await MenuItem.new({
      id: 'id_stop_cue',
      text: t('menu.cue.stop'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyPlaybackCue('stop');
      },
    });

    items.cue.pause = await MenuItem.new({
      id: 'id_pause_cue',
      text: t('menu.cue.pause'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyPlaybackCue('pause');
      },
    });

    items.cue.load = await MenuItem.new({
      id: 'id_load_cue',
      text: t('menu.cue.load'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyPlaybackCue('load');
      },
    });

    items.cue.group = await MenuItem.new({
      id: 'id_group_cue',
      text: t('menu.cue.group'),
      enabled: lastEditEnableStats,
      action: () => {
        const showModel = useShowModel();
        showModel.addEmptyGroupCue();
      },
    });

    submenues.cue = await Submenu.new({
      text: t('menu.cue.title'),
      items: [
        items.cue.audio,
        items.cue.wait,
        items.cue.fade,
        items.cue.start,
        items.cue.stop,
        items.cue.pause,
        items.cue.load,
        items.cue.group,
      ],
    });

    items.tools.renumber = await MenuItem.new({
      id: 'id_renumber',
      text: t('menu.tools.renumber'),
      enabled: lastEditEnableStats,
      action: () => {
        const uiState = useUiState();
        uiState.isRenumberCueDialogOpen = true;
      },
    });

    submenues.tools = await Submenu.new({
      text: t('menu.tools.title'),
      items: [items.tools.renumber],
    });

    let mainHelpMenu: (MenuItem | PredefinedMenuItem)[] = [];
    if (side == 'host') {
      items.help.license = await MenuItem.new({
        id: 'id_license',
        text: t('menu.help.license'),
        action: () => {
          const uiState = useUiState();
          uiState.isLicenseDialogOpen = true;
        },
      });
      mainHelpMenu = [items.help.license];
    }

    items.help.credits = await MenuItem.new({
      id: 'id_credits',
      text: t('menu.help.credits'),
      action: () => {
        const uiState = useUiState();
        uiState.isCreditsDialogOpen = true;
      },
    });

    items.help.showLogFiles = await MenuItem.new({
      id: 'id_show_log_files',
      text: t('menu.help.showLogFiles'),
      action: () => {
        appLogDir().then((path) => {
          openPath(path);
        });
      },
    });

    items.help.checkUpdate = await MenuItem.new({
      id: 'id_check_update',
      text: t('menu.help.checkUpdate'),
      action: () => {
        const uiState = useUiState();
        uiState.isUpdateDialogOpen = true;
      },
    });

    submenues.help = await Submenu.new({
      text: t('menu.help.title'),
      items: [items.help.credits, items.help.showLogFiles, items.help.checkUpdate, ...mainHelpMenu],
    });

    menu = await Menu.new({
      items: [submenues.file, submenues.edit, submenues.cue, submenues.tools, submenues.help],
    });
    menu.setAsAppMenu();
  };

  return { init, updateLocale, updateConnectionStatus, updateEditMode };
};
