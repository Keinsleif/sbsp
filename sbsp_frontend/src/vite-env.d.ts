/// <reference types="vite/client" />
declare const APP_VERSION: string;
declare const __IS_WEBSOCKET__: boolean;
declare const __IS_TAURI__: boolean;
declare const __IS_REMOTE__: boolean;
declare const __IS_HOST__: boolean;

declare module '*.vue' {
  import type { DefineComponent } from 'vue';
  const component: DefineComponent<object, object, unknown>;
  export default component;
}
