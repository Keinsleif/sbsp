import { IBackendAdapter } from './interface';
import { useTauriApi } from './tauri';
import { useWebsocketApi } from './websocket';

const isWebsocket = import.meta.env.VITE_APP_TARGET === 'websocket';

export const useApi: () => IBackendAdapter = isWebsocket ? useWebsocketApi : useTauriApi;
