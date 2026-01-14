import { IBackendAdapter } from './interface';
import { useTauriApi } from './tauri';
import { useWebsocketApi } from './websocket';

export const target = import.meta.env.VITE_APP_TARGET;

export const side: 'host' | 'remote' =
  target == 'websocket' ? 'remote' : import.meta.env.VITE_APP_SIDE == 'host' ? 'host' : 'remote';

export const useApi: () => IBackendAdapter = target == 'websocket' ? useWebsocketApi : useTauriApi;
