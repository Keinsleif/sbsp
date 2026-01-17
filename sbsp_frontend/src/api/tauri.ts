import { Channel, invoke } from '@tauri-apps/api/core';
import { Cue } from '../types/Cue';
import { ShowModel } from '../types/ShowModel';
import { ShowSettings } from '../types/ShowSettings';
import { ShowState } from '../types/ShowState';
import { UiEvent } from '../types/UiEvent';
import { IBackendAdapter, IPickAudioAssetsOptions } from './interface';
import { type } from '@tauri-apps/plugin-os';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { FileList } from '../types/FileList';
import { ServiceEntry } from '../types/ServiceEntry';
import { LicenseInformation } from '../types/LicenseInformation';
import { GlobalSettings } from '../types/GlobalSettings';
import { open, save } from '@tauri-apps/plugin-dialog';
import { i18n } from '../i18n';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useUiState } from '../stores/uistate';
import { ApiServerOptions } from '../types/ApiServerOptions';

const side = import.meta.env.VITE_APP_SIDE;
const { t } = i18n.global;

export function useTauriApi(): IBackendAdapter {
  const tauriApi: IBackendAdapter = {
    host:
      side == 'host'
        ? {
            getLicenseInfo: function (): Promise<LicenseInformation | null> {
              return invoke<LicenseInformation | null>('get_license_info');
            },
            activateLicense: function (): Promise<boolean> {
              return invoke<boolean>('activate_license');
            },

            fileNew: function (): void {
              invoke('file_new').catch((e) => console.error(e));
            },
            fileOpen: function (): void {
              invoke('file_open').catch((e) => console.error(e));
            },
            fileSave: function (): Promise<boolean> {
              return invoke<boolean>('file_save');
            },
            fileSaveAs: function (): Promise<boolean> {
              return invoke<boolean>('file_save_as');
            },
            exportToFolder: function (): Promise<boolean> {
              return invoke<boolean>('export_to_folder');
            },
            isServerRunning: function (): Promise<boolean> {
              return invoke<boolean>('is_server_running');
            },
            setServerOptions: function (options: ApiServerOptions): Promise<void> {
              return invoke('set_server_options', { options: options });
            },
            getServerOptions: function (): Promise<ApiServerOptions> {
              return invoke<ApiServerOptions>('get_server_options');
            },
            getHostname: function (): Promise<string> {
              return invoke<string>('get_hostname');
            },
            startServer: function (): Promise<void> {
              return invoke('start_server');
            },
            stopServer: function (): Promise<void> {
              return invoke('stop_server');
            },
            onServerStatusChanged: function (callback: (status: 'started' | 'stopped') => void): Promise<UnlistenFn> {
              return listen<'started' | 'stopped'>('backend-server-status-changed', (event) => {
                callback(event.payload);
              });
            },
          }
        : undefined,

    remote: {
      isConnected: function (): Promise<boolean> {
        return invoke<boolean>('is_connected');
      },
      getServerAddress: function (): Promise<string | null> {
        return invoke<string | null>('get_server_address');
      },
      connectToServer: function (address: string, password: string | null): Promise<void> {
        return invoke('connect_to_server', { address: address, password: password });
      },
      disconnectFromServer: function (): void {
        invoke('disconnect_from_server').catch((e) => console.error(e));
      },
      startServerDiscovery: function (): void {
        invoke('start_server_discovery').catch((e) => console.error(e));
      },
      stopServerDiscovery: function (): void {
        invoke('stop_server_discovery').catch((e) => console.error(e));
      },
      requestFileList: function (): void {
        invoke('request_file_list').catch((e) => console.error(e));
      },
      onConnectionStatusChanged: function (callback: (isConnected: boolean) => void): Promise<UnlistenFn> {
        return listen<boolean>('connection_status_changed', (event) => {
          callback(event.payload);
        });
      },
      onRemoteDiscoveryUpdate: function (callback: (serviceEntry: ServiceEntry[]) => void): Promise<UnlistenFn> {
        return listen<ServiceEntry[]>('remote-discovery', (entry) => {
          callback(entry.payload);
        });
      },
      onFileListUpdate: function (callback: (fileList: FileList[]) => void): Promise<UnlistenFn> {
        return listen<FileList[]>('asset-list-update', (event) => {
          callback(event.payload);
        });
      },
    },
    isMacOs: function (): boolean {
      return type() == 'macos';
    },
    getThirdPartyNotices: function (): Promise<string> {
      return invoke<string>('get_third_party_notices');
    },
    listenLevelMeter: function (levelListener: (levels: [number, number]) => void): Promise<void> {
      const channel = new Channel<[number, number]>(levelListener);
      return invoke('listen_level_meter', { levelListener: channel });
    },
    pickAudioAssets: function (options: IPickAudioAssetsOptions): Promise<string[]> {
      if (side == 'host') {
        return open({
          multiple: options.multiple,
          directory: false,
          filters: [
            {
              name: 'Audio',
              extensions: [
                'aiff',
                'aif',
                'caf',
                'mp4',
                'm4a',
                'mkv',
                'mka',
                'webm',
                'ogg',
                'oga',
                'wav',
                'aac',
                'alac',
                'flac',
                'mp3',
              ],
            },
          ],
        }).then((paths) => {
          return typeof paths == 'string' ? [paths] : paths || [];
        });
      } else {
        const uiState = useUiState();
        return new Promise((resolve) => {
          uiState.fileListResolver = (select) => {
            resolve(select || []);
          };
          uiState.fileListOption = options.multiple;
        });
      }
    },

    setTitle: function (title: string): void {
      getCurrentWebviewWindow().setTitle(title);
    },

    processAsset: function (path: string): Promise<void> {
      return invoke('process_asset', { path: path });
    },
    setPlaybackCursor: function (cueId: string | null): Promise<void> {
      return invoke('set_playback_cursor', { cueId: cueId });
    },
    sendGo: function (): Promise<void> {
      return invoke('go');
    },
    sendLoad: function (cueId: string): Promise<void> {
      return invoke('load', { cueId: cueId });
    },
    sendPause: function (cueId: string): Promise<void> {
      return invoke('pause', { cueId: cueId });
    },
    sendResume: function (cueId: string): Promise<void> {
      return invoke('resume', { cueId: cueId });
    },
    sendStop: function (cueId: string): Promise<void> {
      return invoke('stop', { cueId: cueId });
    },
    sendPauseAll: function (): Promise<void> {
      return invoke('pause_all');
    },
    sendResumeAll: function (): Promise<void> {
      return invoke('resume_all');
    },
    sendStopAll: function (): Promise<void> {
      return invoke('stop_all');
    },
    sendSeekTo: function (cueId: string, position: number): Promise<void> {
      return invoke('seek_to', { cueId: cueId, position: position });
    },
    sendSeekBy: function (cueId: string, amount: number): Promise<void> {
      return invoke('seek_to', { cueId: cueId, amount: amount });
    },
    sendToggleRepeat: function (cueId: string): Promise<void> {
      return invoke('toggle_repeat', { cueId: cueId });
    },
    sendSetVolume: function (cueId: string, volume: number): Promise<void> {
      return invoke('set_volume', { cueId: cueId, volume: volume });
    },
    getShowModel: function (): Promise<ShowModel> {
      return invoke<ShowModel>('get_show_model');
    },
    isModified: function (): Promise<boolean> {
      return invoke<boolean>('is_modified');
    },
    updateCue: function (cue: Cue): Promise<void> {
      return invoke('update_cue', { cue: cue });
    },
    addCue: function (cue: Cue, targetId: string | null, toBefore: boolean): void {
      invoke('add_cue', { cue: cue, targetId: targetId, toBefore: toBefore }).catch((e) => console.error(e));
    },
    addCues: function (cues: Cue[], targetId: string | null, toBefore: boolean): Promise<void> {
      return invoke('add_cues', { cues: cues, targetId: targetId, toBefore: toBefore });
    },
    removeCue: function (cueId: string): Promise<void> {
      return invoke('remove_cue', { cueId: cueId });
    },
    moveCue: function (cueId: string, targetId: string | null): Promise<void> {
      return invoke('move_cue', { cueId: cueId, targetId: targetId });
    },
    renumberCues: function (cues: string[], startFrom: number, increment: number): Promise<void> {
      return invoke('renumber_cues', { cues: cues, startFrom: startFrom, increment: increment });
    },
    updateModelName: function (newName: string): Promise<void> {
      return invoke('update_model_name', { newName: newName });
    },
    updateShowSettings: function (newSettings: ShowSettings): Promise<void> {
      return invoke('update_show_settings', { newSettings: newSettings });
    },

    getSettings: function (): Promise<GlobalSettings> {
      return invoke<GlobalSettings>('get_settings');
    },
    setSettings: function (newSettings: GlobalSettings): void {
      invoke('set_settings', { newSettings: newSettings }).catch((e) => console.error(e));
    },
    reloadSettings: function (): Promise<GlobalSettings> {
      return invoke<GlobalSettings>('reload_settings');
    },
    saveSettings: function (): void {
      invoke('save_settings', {}).catch((e) => console.error(e));
    },
    importSettingsFromFile: function (): Promise<GlobalSettings> {
      return new Promise((resolve, reject) => {
        open({
          multiple: false,
          directory: false,
        })
          .then((path) => {
            if (path != null) {
              invoke<GlobalSettings>('import_settings_from_file', { path: path }).then(resolve).catch(reject);
            }
          })
          .catch((e) => console.error(e));
      });
    },
    exportSettingsToFile: function (): void {
      save({
        filters: [
          {
            name: t('dialog.save.exportSettingsFilter'),
            extensions: ['json'],
          },
        ],
      })
        .then((path) => {
          if (path != null) {
            invoke('export_settings_to_file', { path: path }).catch((e) => console.error(e));
          }
        })
        .catch((e) => console.error(e));
    },

    onStateUpdate: function (callback: (state: ShowState) => void): Promise<UnlistenFn> {
      return listen<ShowState>('backend-state-update', (event) => {
        callback(event.payload);
      });
    },
    onUiEvent: function (callback: (event: UiEvent) => void): Promise<UnlistenFn> {
      return listen<UiEvent>('backend-event', (event) => {
        callback(event.payload);
      });
    },
  };
  return tauriApi;
}
