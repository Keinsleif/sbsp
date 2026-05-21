import { IBackendAdapter } from './interface';
import { useTauriApi } from './tauri';
import { useWebsocketApi } from './websocket';

export const useApi: () => IBackendAdapter = __IS_WEBSOCKET__ ? useWebsocketApi : useTauriApi;
