// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import type { Cue } from '../types/Cue';
import type { ShowSettings } from '../types/ShowSettings';
import type { BackendEvent } from '../types/BackendEvent';
import { IBackendAdapter, IBackendRemoteAdapter, IPickAudioAssetsOptions, LevelMeterListener } from './interface';
import type { WsFeedback } from '../types/WsFeedback';
import type { FileList } from '../types/FileList';
import type { WsCommand } from '../types/WsCommand';
import type { ProjectStatus } from '../types/ProjectStatus';
import { v4 } from 'uuid';
import type { ServiceEntry } from '../types/ServiceEntry';
import type { GlobalHostSettings } from '../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../types/GlobalRemoteSettings';
import { useUiState } from '../stores/uistate';
import jsSHA from 'jssha';
import type { FullShowState } from '../types/FullShowState';
import type { Permissions } from '../types/Permissions';
import type { BackendError } from '../types/BackendError';
import type { InsertPosition } from '../types/InsertPosition';
import { i18n } from '../i18n';
import { settingsValidator } from '../typia';

const GLOBAL_SETTINGS_STORAGE_KEY = 'sbsp_global_settings';
const { t } = i18n.global;

const DEFAULT_SETTINGS: GlobalHostSettings | GlobalRemoteSettings = {
  general: {
    advanceCursorWhenGo: false,
    lockCursorToSelection: true,
    copyAssetsWhenAdd: false,
    seekAmount: 5,
  },
  appearance: {
    language: null,
    darkMode: 'dark',
    hideControls: false,
  },
  hotkey: {
    playback: {
      go: 'Space',
      load: 'L',
      pauseAndResume: 'P',
      pauseAll: '[',
      resumeAll: ']',
      stop: 'S',
      stopAll: 'Escape',
      seekForward: null,
      seekBackward: null,
    },
    audioAction: {
      toggleRepeat: 'R',
    },
  },
  template: {
    audio: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'audio',
        target: '',
        startTime: null,
        fadeInParam: null,
        endTime: null,
        fadeOutParam: null,
        volume: 0.0,
        pan: 0.0,
        repeat: false,
        soundType: 'streaming',
        envelope: [],
      },
    },
    wait: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'wait',
        duration: 5.0,
      },
    },
    fade: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'fade',
        target: '00000000-0000-0000-0000-000000000000',
        volume: 0.0,
        fadeParam: {
          duration: 3.0,
          easing: {
            type: 'inOutPow',
            intensity: 2,
          },
        },
      },
    },
    start: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'start',
        target: '00000000-0000-0000-0000-000000000000',
      },
    },
    stop: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'stop',
        target: '00000000-0000-0000-0000-000000000000',
        hard: false,
      },
    },
    pause: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'pause',
        target: '00000000-0000-0000-0000-000000000000',
      },
    },
    load: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'load',
        target: '00000000-0000-0000-0000-000000000000',
      },
    },
    group: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      color: 'none',
      preWait: 0,
      chain: {
        type: 'doNotChain',
      },
      params: {
        type: 'group',
        mode: {
          type: 'playlist',
          repeat: true,
        },
        children: [],
      },
    },
  },
  nameFormat: {
    audio: '{filename}',
    wait: 'Wait {duration}',
    fade: 'Fade {targetName}',
    start: 'Start {targetName}',
    stop: 'Stop {targetName}',
    pause: 'Pause {targetName}',
    load: 'Load {targetName}',
    group: 'Group',
  },
};

type UnlistenFn = () => void;

const openFileDialog = (): Promise<globalThis.FileList | null> =>
  new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.multiple = false;
    input.addEventListener('change', () => resolve(input.files));
    input.click();
  });

// const encoder = new TextEncoder();
const strToHashedBase64 = (src: string): string => {
  const shaObj = new jsSHA('SHA-256', 'TEXT');
  shaObj.update(src);
  return shaObj.getHash('B64');
  // const hash = sha256(encoder.encode(src));

  // return btoa(String.fromCharCode(...hash));
};

const websocketApiState: {
  address: string | null;
  permission: Permissions | null;
  ws: WebSocket | null;
  projectStatus: ProjectStatus | null;
  sendQueue: string[];
  backendEventListeners: { [key: string]: (event: BackendEvent) => void };
  assetListListeners: { [key: string]: (list: FileList[]) => void };
  connectionStatusListeners: { [key: string]: (isConnected: boolean, perm: Permissions | null) => void };
  fullStateResolver: [(fullState: FullShowState) => void, () => void] | null;
} = {
  address: null,
  permission: null,
  ws: null,
  projectStatus: null,
  sendQueue: [],
  backendEventListeners: {},
  assetListListeners: {},
  connectionStatusListeners: {},
  fullStateResolver: null,
};

interface IWebsocketBackendAdapter extends IBackendAdapter {
  sendCommand: (command: WsCommand) => void;
  flushQueue: () => void;
}

export function useWebsocketApi(): IBackendAdapter {
  const remoteApi: IBackendRemoteAdapter = {
    isConnected: async function (): Promise<[boolean, Permissions | null]> {
      return [websocketApiState.permission != null, websocketApiState.permission];
    },
    getServerAddress: async function (): Promise<string | null> {
      return websocketApiState.address;
    },
    connectToServer: async function (address: string, password: string | null) {
      if (websocketApiState.ws != null) return;
      let ws: WebSocket;
      websocketApiState.sendQueue = [];
      try {
        ws = new WebSocket(`ws://${address}/ws`);
        websocketApiState.ws = ws;
      } catch (e) {
        console.error(e);
        return;
      }
      let isAuthenticated = false;

      websocketApiState.address = address;
      const closeEventListener = () => {
        console.log('Disconnected.');
        Object.values(websocketApiState.connectionStatusListeners).forEach(cb => cb(false, null));
        ws.removeEventListener('close', closeEventListener);
        ws.removeEventListener('error', errorEventListener);
        if (isAuthenticated) {
          ws.removeEventListener('message', mainEventListener);
        } else {
          ws.removeEventListener('message', authEventListener);
        }
        websocketApiState.ws = null;
        websocketApiState.address = null;
        websocketApiState.permission = null;
      };
      ws.addEventListener('close', closeEventListener);
      const errorEventListener = (event: unknown) => {
        console.error('Websocket error: ', event);
      };
      ws.addEventListener('error', errorEventListener);
      const authEventListener = (e: MessageEvent<string>) => {
        const msg = JSON.parse(e.data) as WsFeedback;
        switch (msg.type) {
          case 'hello': {
            let authString;
            if (password == null) {
              authString = null;
            } else {
              const secret = strToHashedBase64(password + msg.data.auth.salt);
              authString = strToHashedBase64(secret + msg.data.auth.challenge);
            }

            const command: WsCommand = { type: 'authenticate', response: authString };
            ws.send(JSON.stringify(command));
            break;
          }
          case 'authenticated':
            isAuthenticated = true;
            websocketApiState.permission = msg.data.perm;
            ws.addEventListener('message', mainEventListener);
            ws.removeEventListener('message', authEventListener);
            websocketApi.flushQueue();
            Object.values(websocketApiState.connectionStatusListeners).forEach(cb => cb(true, msg.data.perm));
            break;
        }
      };
      const mainEventListener = (e: MessageEvent<string>) => {
        const msg = JSON.parse(e.data) as WsFeedback;
        switch (msg.type) {
          case 'event':
            switch (msg.data.type) {
              case 'showModelLoaded':
                websocketApiState.projectStatus = {
                  status: 'saved',
                  projectType: msg.data.param.projectType,
                  path: msg.data.param.path,
                };
                break;
              case 'showModelReset':
                websocketApiState.projectStatus = {
                  status: 'unsaved',
                };
                break;
              case 'showModelSaved':
                websocketApiState.projectStatus = {
                  status: 'saved',
                  projectType: msg.data.param.projectType,
                  path: msg.data.param.path,
                };
            }
            Object.values(websocketApiState.backendEventListeners).forEach(cb => cb(msg.data));
            break;
          case 'assetList':
            Object.values(websocketApiState.assetListListeners).forEach(cb => cb(msg.data));
            break;
          case 'fullShowState':
            if (websocketApiState.fullStateResolver != null) {
              websocketApiState.fullStateResolver[0](msg.data);
            }
            websocketApiState.projectStatus = msg.data.projectStatus;
            break;
          case 'error': {
            let error: BackendError = {
              type: 'custom',
              id: 0,
              message: 'Unknown error occured.',
            };
            switch (msg.data.type) {
              case 'permissionDenied':
                error = {
                  type: 'custom',
                  id: 2,
                  message: 'Permission Denied.',
                };
                break;
              case 'authenticationFailed':
                error = {
                  type: 'custom',
                  id: 1,
                  message: 'Authentication Failed.',
                };
                break;
            }
            Object.values(websocketApiState.backendEventListeners).forEach(cb => cb({
              type: 'operationFailed',
              param: {
                error: error,
              },
            }));
            break;
          }
        }
      };
      ws.addEventListener('message', authEventListener);
    },
    disconnectFromServer: function (): void {
      websocketApiState.ws?.close();
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    startServerDiscovery: function (_callback: (serviceEntry: ServiceEntry[]) => void): void {
      console.log('Remote discovery on web api is not implemented.');
    },
    stopServerDiscovery: function (): void {
      console.log('Remote discovery on web api is not implemented.');
    },
    requestFileList: function (): void {
      websocketApi.sendCommand({ type: 'requestAssetList' });
    },
    onConnectionStatusChanged: async function (callback: (isConnected: boolean, perm: Permissions | null) => void): Promise<UnlistenFn> {
      const id = v4();
      websocketApiState.connectionStatusListeners[id] = callback;
      return () => {
        delete websocketApiState.connectionStatusListeners[id];
      };
    },
    onFileListUpdate: async function (callback: (fileList: FileList[]) => void): Promise<UnlistenFn> {
      const id = v4();
      websocketApiState.assetListListeners[id] = callback;
      return () => {
        delete websocketApiState.assetListListeners[id];
      };
    },
  };

  const websocketApi: IWebsocketBackendAdapter = {
    sendCommand: function (command: WsCommand): void {
      const ws = websocketApiState.ws;
      if (ws && ws.readyState == WebSocket.OPEN) {
        ws.send(JSON.stringify(command));
      } else if (ws && ws.readyState == WebSocket.CONNECTING) {
        websocketApiState.sendQueue.push(JSON.stringify(command));
      } else {
        console.error('Not connected.');
      }
    },
    flushQueue: function (): void {
      const ws = websocketApiState.ws;
      const sendQueue = websocketApiState.sendQueue;
      if (ws && ws.readyState === WebSocket.OPEN && sendQueue.length > 0) {
        console.log(`Flushing ${sendQueue.length} queued messages...`);
        while (sendQueue.length > 0) {
          const payload = sendQueue.shift();
          if (payload) ws.send(payload);
        }
      }
    },

    remote: remoteApi,
    isMacOs: function (): boolean {
      return navigator?.userAgent?.includes('Macintosh') ?? false;
    },

    requestStateSync() {
      this.sendCommand({ type: 'requestSyncState' });
    },
    getFullState() {
      return new Promise((resolve, reject) => {
        if (websocketApiState.fullStateResolver != null) {
          websocketApiState.fullStateResolver[1]();
        }
        websocketApiState.fullStateResolver = [resolve, reject];
        websocketApi.sendCommand({ type: 'requestFullShowState' });
      });
    },

    getThirdPartyNotices: async function (): Promise<string> {
      return 'Not Available. To read third party notices, please use host app.';
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    listenLevelMeter: function (_levelListener: LevelMeterListener): void {
      console.warn('Not implemented');
    },
    unlistenLevelMeter: function (): void {
      console.warn('Not implemented');
    },
    pickAudioAssets: async function (options: IPickAudioAssetsOptions): Promise<string[]> {
      const uiState = useUiState();
      return new Promise((resolve) => {
        uiState.fileListResolver = (select) => {
          resolve(select || []);
        };
        uiState.fileListOption = options.multiple;
      });
    },

    setTitle: function (title: string): void {
      document.title = title;
    },

    processAsset: async function (path: string): Promise<void> {
      this.sendCommand({ type: 'assetProcessor', command: 'requestFileAssetData', path: path });
    },
    setPlaybackCursor: async function (cueId: string | null): Promise<void> {
      this.sendCommand({ type: 'control', command: 'setPlaybackCursor', params: { cueId: cueId } });
    },
    sendGo: async function (): Promise<void> {
      this.sendCommand({ type: 'control', command: 'go' });
    },
    sendLoad: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'control', command: 'load', params: cueId });
    },
    sendPause: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'control', command: 'pause', params: cueId });
    },
    sendResume: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'control', command: 'resume', params: cueId });
    },
    sendStop: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'control', command: 'stop', params: cueId });
    },
    sendPauseAll: async function (): Promise<void> {
      this.sendCommand({ type: 'control', command: 'pauseAll' });
    },
    sendResumeAll: async function (): Promise<void> {
      this.sendCommand({ type: 'control', command: 'resumeAll' });
    },
    sendStopAll: async function (): Promise<void> {
      this.sendCommand({ type: 'control', command: 'stopAll' });
    },
    sendSeekTo: async function (cueId: string, position: number): Promise<void> {
      this.sendCommand({ type: 'control', command: 'seekTo', params: [cueId, position] });
    },
    sendSeekBy: async function (cueId: string, amount: number): Promise<void> {
      this.sendCommand({ type: 'control', command: 'seekBy', params: [cueId, amount] });
    },
    sendToggleRepeat: async function (cueId: string): Promise<void> {
      this.sendCommand({
        type: 'control',
        command: 'performAction',
        params: [cueId, { type: 'audio', action: 'toggleRepeat' }],
      });
    },
    sendSetVolume: async function (cueId: string, volume: number): Promise<void> {
      this.sendCommand({
        type: 'control',
        command: 'performAction',
        params: [cueId, { type: 'audio', action: 'setVolume', params: volume }],
      });
    },

    isModified: async function (): Promise<boolean> {
      return websocketApiState.projectStatus?.status !== 'saved';
    },
    updateCue: async function (cue: Cue): Promise<void> {
      this.sendCommand({ type: 'model', command: 'updateCue', params: cue });
    },
    addCue: async function (cue: Cue, targetId: string | null, toBefore: boolean): Promise<string> {
      cue.id = v4();
      if (targetId != null) {
        if (toBefore) {
          this.sendCommand({
            type: 'model',
            command: 'addCue',
            params: { cue: cue, position: { type: 'before', target: targetId } },
          });
        } else {
          this.sendCommand({
            type: 'model',
            command: 'addCue',
            params: { cue: cue, position: { type: 'after', target: targetId } },
          });
        }
      } else {
        this.sendCommand({
          type: 'model',
          command: 'addCue',
          params: { cue: cue, position: { type: 'last' } },
        });
      }
      return cue.id;
    },
    addCues: async function (cues: Cue[], targetId: string | null, toBefore: boolean): Promise<string[]> {
      const cueIds = cues.map((cue) => {
        cue.id = v4();
        return cue.id;
      });
      if (targetId != null) {
        if (toBefore) {
          this.sendCommand({
            type: 'model',
            command: 'addCues',
            params: { cues: cues, position: { type: 'before', target: targetId } },
          });
        } else {
          this.sendCommand({
            type: 'model',
            command: 'addCues',
            params: { cues: cues, position: { type: 'after', target: targetId } },
          });
        }
      } else {
        this.sendCommand({
          type: 'model',
          command: 'addCues',
          params: { cues: cues, position: { type: 'last' } },
        });
      }
      return cueIds;
    },
    removeCue: async function (cueId: string, confirm_remove: boolean = true): Promise<void> {
      if (confirm_remove) {
        const removeOk = confirm(t('dialog.message.removeCue'));
        if (!removeOk) {
          return;
        }
      }
      this.sendCommand({ type: 'model', command: 'removeCue', params: { cueId: cueId } });
    },
    removeCues: async function (cueIds: string[], confirm_remove: boolean = true): Promise<void> {
      if (cueIds.length === 0) {
        return;
      }
      if (confirm_remove) {
        const removeOk = confirm(t('dialog.message.removeCue'));
        if (!removeOk) {
          return;
        }
      }
      this.sendCommand({ type: 'model', command: 'removeCues', params: { cueIds: cueIds } });
    },
    moveCue: async function (cueId: string, position: InsertPosition): Promise<void> {
      this.sendCommand({
        type: 'model',
        command: 'moveCue',
        params: { cueId: cueId, position: position },
      });
    },
    moveCues: async function (cueIds: string[], position: InsertPosition): Promise<void> {
      this.sendCommand({
        type: 'model',
        command: 'moveCues',
        params: { cueIds, position: position },
      });
    },
    renumberCues: async function (cues: string[], startFrom: number, increment: number, prefix: string | null, suffix: string | null): Promise<void> {
      this.sendCommand({
        type: 'model',
        command: 'renumberCues',
        params: { cues, startFrom, increment, prefix, suffix },
      });
    },
    updateModelName: async function (newName: string): Promise<void> {
      this.sendCommand({ type: 'model', command: 'updateModelName', params: newName });
    },
    updateShowSettings: async function (newSettings: ShowSettings): Promise<void> {
      this.sendCommand({ type: 'model', command: 'updateSettings', params: newSettings });
    },

    getSettings: async function (): Promise<GlobalHostSettings | GlobalRemoteSettings> {
      const settings = localStorage.getItem(GLOBAL_SETTINGS_STORAGE_KEY);
      if (settings != null) {
        return JSON.parse(settings) as GlobalHostSettings | GlobalRemoteSettings;
      } else {
        localStorage.setItem(GLOBAL_SETTINGS_STORAGE_KEY, JSON.stringify(DEFAULT_SETTINGS));
        return DEFAULT_SETTINGS;
      }
    },
    setSettings: function (newSettings: GlobalHostSettings | GlobalRemoteSettings): void {
      localStorage.setItem(GLOBAL_SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    },
    reloadSettings: function (): Promise<GlobalHostSettings | GlobalRemoteSettings> {
      return this.getSettings();
    },
    importSettingsFromFile: async function (): Promise<GlobalHostSettings | GlobalRemoteSettings> {
      return new Promise<GlobalHostSettings | GlobalRemoteSettings>((resolve, reject) => {
        openFileDialog()
          .then((files) => {
            const filepath = files != null ? files[0] : null;
            if (filepath == null) {
              reject();
              return;
            }
            filepath.text().then((text) => {
              try {
                const result = settingsValidator(JSON.parse(text));
                if (result.success) {
                  resolve(result.data);
                } else {
                  reject(result.errors);
                }
                // resolve(JSON.parse(text));
              } catch {
                reject();
              }
            });
          })
          .catch(reject);
      });
    },
    exportSettingsToFile: function (): void {
      this.getSettings().then((settings) => {
        const blob = new Blob([JSON.stringify(settings, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'sbsp_config.json';
        a.click();
        URL.revokeObjectURL(url);
      });
    },

    onBackendEvent: async function (callback: (event: BackendEvent) => void): Promise<UnlistenFn> {
      const id = v4();
      websocketApiState.backendEventListeners[id] = callback;
      return () => {
        delete websocketApiState.backendEventListeners[id];
      };
    },
  };
  return websocketApi;
}
