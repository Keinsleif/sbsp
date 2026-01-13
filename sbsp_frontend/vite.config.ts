import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import vuetify from 'vite-plugin-vuetify';
import VueI18nPlugin from '@intlify/unplugin-vue-i18n/vite';
import { visualizer } from 'rollup-plugin-visualizer';
import UnpluginTyia from '@kennethwkz/unplugin-typia/vite';

const host = process.env.TAURI_DEV_HOST;

const outDir = process.env.VITE_APP_SIDE
  ? `dist/${process.env.VITE_APP_TARGET}/${process.env.VITE_APP_SIDE}`
  : `dist/${process.env.VITE_APP_TARGET}`;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    UnpluginTyia({ cache: true }),
    vuetify({
      styles: {
        configFile: 'src/styles/settings.scss',
      },
    }),
    VueI18nPlugin(),
    visualizer(),
  ],

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
  build: {
    outDir: outDir,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules')) {
            if (id.includes('vuetify')) {
              return 'vendor-vuetify';
            }
            return 'vendor';
          }
        },
      },
    },
    target: ['es2022', 'chrome89', 'safari15'],
  },
}));
