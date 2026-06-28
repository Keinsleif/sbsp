// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { Channel, invoke } from '@tauri-apps/api/core';
import type { Cue } from '../types/Cue';
import type { ShowSettings } from '../types/ShowSettings';
import type { BackendEvent } from '../types/BackendEvent';
import type {
  BackendEventListener,
  IBackendAdapter,
  IPickAudioAssetsOptions,
  LevelMeterListener,
} from './interface';
import { type } from '@tauri-apps/plugin-os';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { FileList } from '../types/FileList';
import type { ServiceEntry } from '../types/ServiceEntry';
import type { LicenseInformation } from '../types/LicenseInformation';
import type { GlobalHostSettings } from '../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../types/GlobalRemoteSettings';
import { message, open, save } from '@tauri-apps/plugin-dialog';
import { i18n } from '../i18n';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useUiState } from '../stores/uiState';
import type { ApiServerOptions } from '../types/ApiServerOptions';
import type { FullShowState } from '../types/FullShowState';
import type { SupportedHardware } from '../types/SupportedHardware';
import type { Permissions } from '../types/Permissions';
import type { InsertPosition } from '../types/InsertPosition';
import { v4 } from 'uuid';

const { t } = i18n.global;

export function useTauriApi(): IBackendAdapter {
  const tauriApi: IBackendAdapter = {
    host: __IS_HOST__
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
          getHardware: function (): Promise<SupportedHardware> {
            return invoke('get_hardware');
          },
          onServerStatusChanged: function (
            callback: (status: 'started' | 'stopped') => void,
          ): Promise<UnlistenFn> {
            return listen<'started' | 'stopped'>('backend-server-status-changed', (event) => {
              callback(event.payload);
            });
          },
        }
      : undefined,

    remote: __IS_REMOTE__
      ? {
          isConnected: function (): Promise<[boolean, Permissions | null]> {
            return invoke<[boolean, Permissions | null]>('is_connected');
          },
          getServerAddress: function (): Promise<string | null> {
            return invoke<string | null>('get_server_address');
          },
          connectToServer: function (address: string, password: string | null): Promise<void> {
            return invoke<void>('connect_to_server', { address: address, password: password });
          },
          disconnectFromServer: function (): void {
            invoke('disconnect_from_server').catch((e) => console.error(e));
          },
          startServerDiscovery: function (callback: (serviceEntry: ServiceEntry[]) => void): void {
            const channel = new Channel<ServiceEntry[]>(callback);
            invoke('start_server_discovery', { channel }).catch((e) => console.error(e));
          },
          stopServerDiscovery: function (): void {
            invoke('stop_server_discovery').catch((e) => console.error(e));
          },
          requestFileList: function (): void {
            invoke('request_file_list').catch((e) => console.error(e));
          },
          onConnectionStatusChanged: function (
            callback: (isConnected: boolean, perm: Permissions | null) => void,
          ): Promise<UnlistenFn> {
            return listen<[boolean, Permissions | null]>('connection_status_changed', (event) => {
              callback(event.payload[0], event.payload[1]);
            });
          },
          onFileListUpdate: function (
            callback: (fileList: FileList[]) => void,
          ): Promise<UnlistenFn> {
            return listen<FileList[]>('asset-list-update', (event) => {
              callback(event.payload);
            });
          },
        }
      : undefined,
    isMacOs: function (): boolean {
      return type() === 'macos';
    },

    requestStateSync() {
      invoke('request_state_sync');
    },
    getFullState() {
      return invoke<FullShowState>('get_full_state');
    },

    getThirdPartyNotices: function (): Promise<string> {
      return invoke<string>('get_third_party_notices');
    },
    listenLevelMeter: function (levelListener: LevelMeterListener): void {
      const channel = new Channel<ArrayBuffer>((value) => {
        if (value.byteLength !== 8) {
          return; // ignore invalid ipc value
        }
        const dv = new DataView(value);
        levelListener([dv.getFloat32(0, true), dv.getFloat32(4, true)]);
      });
      invoke('listen_level_meter', { levelListener: channel });
    },
    unlistenLevelMeter: function (): void {
      invoke('unlisten_level_meter');
    },
    pickAudioAssets: async function (options: IPickAudioAssetsOptions): Promise<string[]> {
      if (__IS_HOST__) {
        const paths = await open({
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
        });
        return typeof paths === 'string' ? [paths] : paths || [];
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
      return invoke('seek_by', { cueId: cueId, amount: amount });
    },
    sendToggleRepeat: function (cueId: string): Promise<void> {
      return invoke('toggle_repeat', { cueId: cueId });
    },
    sendSetVolume: function (cueId: string, volume: number): Promise<void> {
      return invoke('set_volume', { cueId: cueId, volume: volume });
    },

    isModified: function (): Promise<boolean> {
      return invoke<boolean>('is_modified');
    },
    updateCue: function (cue: Cue): Promise<void> {
      return invoke('update_cue', { cue: cue });
    },
    addCue: async function (cue: Cue, targetId: string | null, toBefore: boolean): Promise<string> {
      cue.id = v4();
      await invoke('add_cue', { cue: cue, targetId: targetId, toBefore: toBefore });
      return cue.id;
    },
    addCues: async function (
      cues: Cue[],
      targetId: string | null,
      toBefore: boolean,
    ): Promise<string[]> {
      const cueIds = cues.map((cue) => {
        cue.id = v4();
        return cue.id;
      });
      await invoke('add_cues', { cues: cues, targetId: targetId, toBefore: toBefore });
      return cueIds;
    },
    removeCue: async function (cueId: string, confirm_remove: boolean = true) {
      if (confirm_remove) {
        const removeOk = await message(t('dialog.message.removeCue'), {
          title: t('dialog.message.confirmation'),
          kind: 'warning',
          buttons: 'OkCancel',
        });
        if (removeOk !== 'Ok') {
          return;
        }
      }
      await invoke('remove_cue', { cueId: cueId });
    },
    removeCues: async function (cueIds: string[], confirm_remove: boolean = true) {
      if (cueIds.length === 0) {
        return;
      }
      if (confirm_remove) {
        const removeOk = await message(t('dialog.message.removeCue'), {
          title: t('dialog.message.confirmation'),
          kind: 'warning',
          buttons: 'OkCancel',
        });
        if (removeOk !== 'Ok') {
          return;
        }
      }
      await invoke('remove_cues', { cueIds: cueIds });
    },
    moveCue: function (cueId: string, position: InsertPosition): Promise<void> {
      return invoke('move_cue', { cueId: cueId, position: position });
    },
    moveCues: function (cueIds: string[], position: InsertPosition): Promise<void> {
      return invoke('move_cues', { cueIds: cueIds, position: position });
    },
    renumberCues: function (
      cues: string[],
      startFrom: number,
      increment: number,
      prefix: string | null,
      suffix: string | null,
    ): Promise<void> {
      return invoke('renumber_cues', { cues, startFrom, increment, prefix, suffix });
    },
    updateModelName: function (newName: string): Promise<void> {
      return invoke('update_model_name', { newName: newName });
    },
    updateShowSettings: function (newSettings: ShowSettings): Promise<void> {
      return invoke('update_show_settings', { newSettings: newSettings });
    },

    getSettings: function (): Promise<GlobalHostSettings | GlobalRemoteSettings> {
      return invoke<GlobalHostSettings | GlobalRemoteSettings>('get_settings');
    },
    setSettings: function (newSettings: GlobalHostSettings | GlobalRemoteSettings): void {
      invoke('set_settings', { newSettings: newSettings }).catch((e) => console.error(e));
    },
    reloadSettings: function (): Promise<GlobalHostSettings | GlobalRemoteSettings> {
      return invoke<GlobalHostSettings | GlobalRemoteSettings>('reload_settings');
    },
    importSettingsFromFile: function (): Promise<GlobalHostSettings | GlobalRemoteSettings> {
      return new Promise((resolve, reject) => {
        open({
          multiple: false,
          directory: false,
        })
          .then((path) => {
            if (path != null) {
              invoke<GlobalHostSettings | GlobalRemoteSettings>('import_settings_from_file', {
                path: path,
              })
                .then(resolve)
                .catch(reject);
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

    onBackendEvent: function (callback: BackendEventListener): Promise<UnlistenFn> {
      const channel = new Channel<BackendEvent>(callback);
      invoke('listen_backend_event', { channel }).catch((e) => console.error(e));
      return new Promise((resolve) =>
        resolve(() => {
          invoke('unlisten_backend_event').catch((e) => console.error(e));
        }),
      );
    },
  };
  return tauriApi;
}
