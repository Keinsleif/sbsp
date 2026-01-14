import { Cue } from '../types/Cue';
import { ShowModel } from '../types/ShowModel';
import { ShowSettings } from '../types/ShowSettings';
import { ShowState } from '../types/ShowState';
import { UiEvent } from '../types/UiEvent';
import { IBackendAdapter, IBackendRemoteAdapter, IPickAudioAssetsOptions } from './interface';
import { WsFeedback } from '../types/WsFeedback';
import { FileList } from '../types/FileList';
import { WsCommand } from '../types/WsCommand';
import { ProjectStatus } from '../types/ProjectStatus';
import { v4 } from 'uuid';
import { ServiceEntry } from '../types/ServiceEntry';
import { LicenseInformation } from '../types/LicenseInformation';
import { GlobalSettings } from '../types/GlobalSettings';
import typia from 'typia';
import { useUiState } from '../stores/uistate';

const GLOBAL_SETTINGS_STORAGE_KEY = 'sbsp_global_settings';

const DEFAULT_SETTINGS: GlobalSettings = {
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
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
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
      },
    },
    wait: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
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
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
      },
      params: {
        type: 'fade',
        target: '00000000-0000-0000-0000-000000000000',
        volume: 0.0,
        fadeParam: {
          duration: 3.0,
          easing: {
            type: 'inOutPowi',
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
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
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
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
      },
      params: {
        type: 'stop',
        target: '00000000-0000-0000-0000-000000000000',
      },
    },
    pause: {
      id: '00000000-0000-0000-0000-000000000000',
      number: '',
      name: null,
      notes: '',
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
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
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
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
      preWait: 0,
      sequence: {
        type: 'doNotContinue',
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

const settingsValidator = typia.createValidate<GlobalSettings>();

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

const websocketApiState: {
  address: string | null;
  ws: WebSocket | null;
  showModelBuffer: ShowModel | null;
  projectStatus: ProjectStatus | null;
  stateUpdateListeners: { [key: string]: (state: ShowState) => void };
  uiEventListeners: { [key: string]: (event: UiEvent) => void };
  assetListListeners: { [key: string]: (list: FileList[]) => void };
  connectionStatusListeners: { [key: string]: (isConnected: boolean) => void };
} = {
  address: null,
  ws: null,
  showModelBuffer: null,
  projectStatus: null,
  stateUpdateListeners: {},
  uiEventListeners: {},
  assetListListeners: {},
  connectionStatusListeners: {},
};

interface IWebsocketBackendAdapter extends IBackendAdapter {
  sendCommand: (command: WsCommand) => void;
}

export function useWebsocketApi(): IBackendAdapter {
  const remoteApi: IBackendRemoteAdapter = {
    isConnected: async function (): Promise<boolean> {
      return websocketApiState.address != null;
    },
    getServerAddress: async function (): Promise<string | null> {
      return websocketApiState.address;
    },
    connectToServer: function (address: string) {
      return new Promise((resolve, reject) => {
        try {
          websocketApiState.ws = new WebSocket(`ws://${address}/ws`);
        } catch (e) {
          reject(e);
          return;
        }
        websocketApiState.ws.onmessage = (e) => {
          const msg = JSON.parse(e.data) as WsFeedback;
          switch (msg.type) {
            case 'state':
              Object.values(websocketApiState.stateUpdateListeners).forEach((cb) => cb(msg.data));
              break;
            case 'event':
              switch (msg.data.type) {
                case 'showModelLoaded':
                case 'showModelReset':
                  websocketApi.sendCommand({ type: 'requestFullShowState' });
                  break;
                case 'showModelSaved':
                  websocketApiState.projectStatus = {
                    status: 'saved',
                    projectType: msg.data.param.projectType,
                    path: msg.data.param.path,
                  };
              }
              Object.values(websocketApiState.uiEventListeners).forEach((cb) => cb(msg.data));
              break;
            case 'assetList':
              Object.values(websocketApiState.assetListListeners).forEach((cb) => cb(msg.data));
              break;
            case 'fullShowState':
              websocketApiState.showModelBuffer = msg.data.showModel;
              websocketApiState.projectStatus = msg.data.projectStatus;
          }
        };
        websocketApiState.ws.onclose = () => {
          Object.values(websocketApiState.connectionStatusListeners).forEach((cb) => cb(false));
          websocketApiState.ws = null;
          websocketApiState.address = null;
        };
        websocketApiState.ws.onopen = () => {
          websocketApiState.address = address;
          Object.values(websocketApiState.connectionStatusListeners).forEach((cb) => cb(true));
          resolve();
        };
      });
    },
    disconnectFromServer: function (): void {
      websocketApiState.ws?.close();
    },
    startServerDiscovery: function (): void {
      console.log('Remote discovery on web api is not implemented.');
    },
    stopServerDiscovery: function (): void {
      console.log('Remote discovery on web api is not implemented.');
    },
    requestFileList: function (): void {
      websocketApi.sendCommand({ type: 'requestAssetList' });
    },
    onConnectionStatusChanged: async function (callback: (isConnected: boolean) => void): Promise<UnlistenFn> {
      const id = v4();
      websocketApiState.connectionStatusListeners[id] = callback;
      return () => {
        delete websocketApiState.connectionStatusListeners[id];
      };
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    onRemoteDiscoveryUpdate: async function (_callback: (serviceEntry: ServiceEntry[]) => void): Promise<UnlistenFn> {
      console.log('Remote discovery on web api is not implemented.');
      return () => {};
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
      if (websocketApiState.ws) {
        websocketApiState.ws.send(JSON.stringify(command));
      }
    },
    side: 'remote',
    target: 'websocket',
    remote: remoteApi,
    isMacOs: function (): boolean {
      return navigator?.userAgent?.includes('Macintosh') ?? false;
    },
    getThirdPartyNotices: async function (): Promise<string> {
      return 'Not Available. To read third party notices, please use host app.';
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    listenLevelMeter: async function (_levelListener: (levels: [number, number]) => void): Promise<void> {
      console.error('Not implemented');
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
    getLicenseInfo: function (): Promise<LicenseInformation | null> {
      return new Promise<LicenseInformation | null>((resolve) => resolve(null));
    },
    activateLicense: async function (): Promise<boolean> {
      return false;
    },

    setTitle: function (title: string): void {
      document.title = title;
    },

    processAsset: async function (path: string): Promise<void> {
      this.sendCommand({ type: 'assetProcessor', command: 'requestFileAssetData', path: path });
    },
    setPlaybackCursor: async function (cueId: string | null): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'setPlaybackCursor', params: { cueId: cueId } });
    },
    sendGo: async function (): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'go' });
    },
    sendLoad: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'load', params: cueId });
    },
    sendPause: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'pause', params: cueId });
    },
    sendResume: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'resume', params: cueId });
    },
    sendStop: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'stop', params: cueId });
    },
    sendPauseAll: async function (): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'pauseAll' });
    },
    sendResumeAll: async function (): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'resumeAll' });
    },
    sendStopAll: async function (): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'stopAll' });
    },
    sendSeekTo: async function (cueId: string, position: number): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'seekTo', params: [cueId, position] });
    },
    sendSeekBy: async function (cueId: string, amount: number): Promise<void> {
      this.sendCommand({ type: 'controll', command: 'seekBy', params: [cueId, amount] });
    },
    sendToggleRepeat: async function (cueId: string): Promise<void> {
      this.sendCommand({
        type: 'controll',
        command: 'performAction',
        params: [cueId, { type: 'audio', action: 'toggleRepeat' }],
      });
    },
    sendSetVolume: async function (cueId: string, volume: number): Promise<void> {
      this.sendCommand({
        type: 'controll',
        command: 'performAction',
        params: [cueId, { type: 'audio', action: 'setVolume', params: volume }],
      });
    },
    getShowModel: function (): Promise<ShowModel> {
      return new Promise((resolve) => {
        if (websocketApiState.showModelBuffer != null) {
          resolve(websocketApiState.showModelBuffer);
        } else {
          const initShowModelListener = (e: MessageEvent<string>) => {
            const wsFeedback = JSON.parse(e.data) as WsFeedback;
            if (wsFeedback.type == 'fullShowState') {
              websocketApiState.showModelBuffer = wsFeedback.data.showModel;
              resolve(wsFeedback.data.showModel);
              websocketApiState.ws?.removeEventListener('message', initShowModelListener);
            }
          };
          websocketApiState.ws?.addEventListener('message', initShowModelListener);
          websocketApi.sendCommand({ type: 'requestFullShowState' });
        }
      });
    },
    isModified: async function (): Promise<boolean> {
      return websocketApiState.projectStatus?.status !== 'saved';
    },
    updateCue: async function (cue: Cue): Promise<void> {
      this.sendCommand({ type: 'model', command: 'updateCue', params: cue });
    },
    addCue: function (cue: Cue, targetId: string | null, toBefore: boolean): void {
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
    },
    addCues: async function (cues: Cue[], targetId: string | null, toBefore: boolean): Promise<void> {
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
    },
    removeCue: async function (cueId: string): Promise<void> {
      this.sendCommand({ type: 'model', command: 'removeCue', params: { cueId: cueId } });
    },
    moveCue: async function (cueId: string, targetId: string | null): Promise<void> {
      if (targetId != null) {
        this.sendCommand({
          type: 'model',
          command: 'moveCue',
          params: { cueId: cueId, position: { type: 'before', target: targetId } },
        });
      } else {
        this.sendCommand({
          type: 'model',
          command: 'moveCue',
          params: { cueId: cueId, position: { type: 'last' } },
        });
      }
    },
    renumberCues: async function (cues: string[], startFrom: number, increment: number): Promise<void> {
      this.sendCommand({
        type: 'model',
        command: 'renumberCues',
        params: { cues: cues, startFrom: startFrom, increment: increment },
      });
    },
    updateModelName: async function (newName: string): Promise<void> {
      this.sendCommand({ type: 'model', command: 'updateModelName', params: newName });
    },
    updateShowSettings: async function (newSettings: ShowSettings): Promise<void> {
      this.sendCommand({ type: 'model', command: 'updateSettings', params: newSettings });
    },

    getSettings: async function (): Promise<GlobalSettings> {
      const settings = localStorage.getItem(GLOBAL_SETTINGS_STORAGE_KEY);
      if (settings != null) {
        return JSON.parse(settings) as GlobalSettings;
      } else {
        localStorage.setItem(GLOBAL_SETTINGS_STORAGE_KEY, JSON.stringify(DEFAULT_SETTINGS));
        return DEFAULT_SETTINGS;
      }
    },
    setSettings: function (newSettings: GlobalSettings): void {
      localStorage.setItem(GLOBAL_SETTINGS_STORAGE_KEY, JSON.stringify(newSettings));
    },
    reloadSettings: function (): Promise<GlobalSettings> {
      return this.getSettings();
    },
    saveSettings: function (): void {
      return;
    },
    importSettingsFromFile: async function (): Promise<GlobalSettings> {
      return new Promise<GlobalSettings>((resolve, reject) => {
        openFileDialog()
          .then((files) => {
            if (files == null || files.length != 1) {
              reject();
              return;
            }
            files[0].text().then((text) => {
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

    onStateUpdate: async function (callback: (state: ShowState) => void): Promise<UnlistenFn> {
      const id = v4();
      websocketApiState.stateUpdateListeners[id] = callback;
      return () => {
        delete websocketApiState.stateUpdateListeners[id];
      };
    },
    onUiEvent: async function (callback: (event: UiEvent) => void): Promise<UnlistenFn> {
      const id = v4();
      websocketApiState.uiEventListeners[id] = callback;
      return () => {
        delete websocketApiState.uiEventListeners[id];
      };
    },
  };
  return websocketApi;
}
