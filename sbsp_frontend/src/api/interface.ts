import type { Cue } from '../types/Cue';
import type { BackendEvent } from '../types/BackendEvent';
import type { ShowSettings } from '../types/ShowSettings';
import type { FileList } from '../types/FileList';
import type { ServiceEntry } from '../types/ServiceEntry';
import type { LicenseInformation } from '../types/LicenseInformation';
import type { GlobalHostSettings } from '../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../types/GlobalRemoteSettings';
import type { ApiServerOptions } from '../types/ApiServerOptions';
import type { FullShowState } from '../types/FullShowState';
import type { SupportedHardware } from '../types/SupportedHardware';

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

  // state sync
  requestStateSync(): void;
  getFullState(): Promise<FullShowState>;

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
  isModified(): Promise<boolean>;

  // Model commands
  updateCue(cue: Cue): Promise<void>;
  addCue(cue: Cue, targetId: string | null, toBefore: boolean): void;
  addCues(cues: Cue[], targetId: string | null, toBefore: boolean): Promise<void>;
  removeCue(cueId: string): Promise<void>;
  moveCue(cueId: string, targetId: string | null): Promise<void>;
  renumberCues(cues: string[], startFrom: number, increment: number, prefix: string | null, suffix: string | null): Promise<void>;
  updateModelName(newName: string): Promise<void>;
  updateShowSettings(newSettings: ShowSettings): Promise<void>;

  // Settings
  getSettings(): Promise<GlobalHostSettings | GlobalRemoteSettings>;
  setSettings(newSettings: GlobalHostSettings | GlobalRemoteSettings): void;
  reloadSettings(): Promise<GlobalHostSettings | GlobalRemoteSettings>;
  importSettingsFromFile(): Promise<GlobalHostSettings | GlobalRemoteSettings>;
  exportSettingsToFile(): void;

  onBackendEvent(callback: (event: BackendEvent) => void): Promise<UnlistenFn>;
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

  getHardware(): Promise<SupportedHardware>;

  onServerStatusChanged(callback: (status: 'started' | 'stopped') => void): Promise<UnlistenFn>;
}

export interface IBackendRemoteAdapter {
  // Client Specific
  isConnected(): Promise<boolean>;
  getServerAddress(): Promise<string | null>;
  connectToServer(address: string, password: string | null): Promise<void>;
  disconnectFromServer(): void;
  startServerDiscovery(callback: (serviceEntry: ServiceEntry[]) => void): void;
  stopServerDiscovery(): void;
  requestFileList(): void;

  onConnectionStatusChanged(callback: (isConnected: boolean) => void): Promise<UnlistenFn>;
  onFileListUpdate(callback: (fileList: FileList[]) => void): Promise<UnlistenFn>;
}
