// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { fileURLToPath, URL } from 'node:url';

import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import vueDevTools from 'vite-plugin-vue-devtools';
import UnpluginTypia from '@typia/unplugin/vite';
import tailwindcss from '@tailwindcss/vite';

const host = process.env.TAURI_DEV_HOST;

const outDir = process.env.VITE_APP_SIDE
  ? `dist/${process.env.VITE_APP_TARGET}/${process.env.VITE_APP_SIDE}`
  : `dist/${process.env.VITE_APP_TARGET}`;

  // https://vite.dev/config/
export default defineConfig({
  plugins: [
    tailwindcss(),
    vue(),
    UnpluginTypia({ cache: true }),
    vueDevTools(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  define: {
    APP_VERSION: JSON.stringify(process.env.npm_package_version),
    __IS_WEBSOCKET__: JSON.stringify(process.env.VITE_APP_TARGET === 'websocket'),
    __IS_TAURI__: JSON.stringify(process.env.VITE_APP_TARGET === 'tauri'),
    __IS_REMOTE__: JSON.stringify(process.env.VITE_APP_TARGET === 'websocket' || process.env.VITE_APP_SIDE === 'remote'),
    __IS_HOST__: JSON.stringify(process.env.VITE_APP_TARGET !== 'websocket' && process.env.VITE_APP_SIDE === 'host'),
  },
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    outDir: outDir,
    rolldownOptions: {
      output: {
        manualChunks(id: string | string[]) {
          if (id.includes('node_modules')) {
            if (id.includes('vuetify')) {
              return 'vendor-vuetify';
            }
            return 'vendor';
          }
        },
      },
    },
    target: process.env.VITE_APP_TARGET === 'tauri'
      ? process.env.TAURI_ENV_PLATFORM === 'windows'
        ? 'chrome105'
        : 'safari13'
      : 'baseline-widely-available',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
