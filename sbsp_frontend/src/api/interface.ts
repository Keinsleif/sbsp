import type { Cue } from '../types/Cue';
import type { ShowModel } from '../types/ShowModel';
import type { ShowState } from '../types/ShowState';
import type { UiEvent } from '../types/UiEvent';
import { ShowSettings } from '../types/ShowSettings';
import { FileList } from '../types/FileList';
import { ServiceEntry } from '../types/ServiceEntry';
import { LicenseInformation } from '../types/LicenseInformation';
import { GlobalSettings } from '../types/GlobalSettings';
import { ApiServerOptions } from '../types/ApiServerOptions';

type UnlistenFn = () => void;

export interface IPickAudioAssetsOptions {
  multiple: boolean;
}

export interface IBackendAdapter {
  host?: IBackendHostAdapter;
  remote?: IBackendRemoteAdapter;

  isMacOs(): boolean;
  getThirdPartyNotices(): Promise<string>;
  listenLevelMeter(levelListener: (levels: [number, number]) => void): Promise<void>;
  pickAudioAssets(options: IPickAudioAssetsOptions): Promise<string[]>;

  setTitle(title: string): void;

  // asset processor
  processAsset(path: string): Promise<void>;

  // controller commands
  setPlaybackCursor(cueId: string | null): Promise<void>;
  sendGo(): Promise<void>;
  sendLoad(cueId: string): Promise<void>;
  sendPause(cueId: string): Promise<void>;
  sendResume(cueId: string): Promise<void>;
  sendStop(cueId: string): Promise<void>;
  sendPauseAll(): Promise<void>;
  sendResumeAll(): Promise<void>;
  sendStopAll(): Promise<void>;
  sendSeekTo(cueId: string, position: number): Promise<void>;
  sendSeekBy(cueId: string, amount: number): Promise<void>;
  sendToggleRepeat(cueId: string): Promise<void>;
  sendSetVolume(cueId: string, volume: number): Promise<void>;

  // Model getter
  getShowModel(): Promise<ShowModel>;
  isModified(): Promise<boolean>;
  // Model commands
  updateCue(cue: Cue): Promise<void>;
  addCue(cue: Cue, targetId: string | null, toBefore: boolean): void;
  addCues(cues: Cue[], targetId: string | null, toBefore: boolean): Promise<void>;
  removeCue(cueId: string): Promise<void>;
  moveCue(cueId: string, targetId: string | null): Promise<void>;
  renumberCues(cues: string[], startFrom: number, increment: number): Promise<void>;
  updateModelName(newName: string): Promise<void>;
  updateShowSettings(newSettings: ShowSettings): Promise<void>;

  // Settings
  getSettings(): Promise<GlobalSettings>;
  setSettings(newSettings: GlobalSettings): void;
  reloadSettings(): Promise<GlobalSettings>;
  saveSettings(): void;
  importSettingsFromFile(): Promise<GlobalSettings>;
  exportSettingsToFile(): void;

  onStateUpdate(callback: (state: ShowState) => void): Promise<UnlistenFn>;
  onUiEvent(callback: (event: UiEvent) => void): Promise<UnlistenFn>;
}

export interface IBackendHostAdapter {
  getLicenseInfo(): Promise<LicenseInformation | null>;
  activateLicense(): Promise<boolean>;

  // file pick
  fileNew(): void;
  fileOpen(): void;
  fileSave(): Promise<boolean>;
  fileSaveAs(): Promise<boolean>;
  exportToFolder(): Promise<boolean>;

  // Server Specific
  isServerRunning(): Promise<boolean>;
  setServerOptions(options: ApiServerOptions): Promise<void>;
  getServerOptions(): Promise<ApiServerOptions>;
  getHostname(): Promise<string>;
  startServer(): Promise<void>;
  stopServer(): Promise<void>;

  onServerStatusChanged(callback: (status: 'started' | 'stopped') => void): Promise<UnlistenFn>;
}

export interface IBackendRemoteAdapter {
  // Client Specific
  isConnected(): Promise<boolean>;
  getServerAddress(): Promise<string | null>;
  connectToServer(address: string, password: string | null): Promise<void>;
  disconnectFromServer(): void;
  startServerDiscovery(): void;
  stopServerDiscovery(): void;
  requestFileList(): void;

  onConnectionStatusChanged(callback: (isConnected: boolean) => void): Promise<UnlistenFn>;
  onRemoteDiscoveryUpdate(callback: (serviceEntry: ServiceEntry[]) => void): Promise<UnlistenFn>;
  onFileListUpdate(callback: (fileList: FileList[]) => void): Promise<UnlistenFn>;
}
