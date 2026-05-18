import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],

  build: {
    rolldownOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) {
            return;
          }

          if (id.includes('/naive-ui/')) {
            if (
              id.includes('/_internal/') ||
              id.includes('/_utils/') ||
              id.includes('/_styles/') ||
              id.includes('/composables/') ||
              id.includes('/global-style/') ||
              id.includes('/locales/') ||
              id.includes('/config-provider/') ||
              id.includes('/discrete/') ||
              id.includes('/message/')
            ) {
              return 'naive-core';
            }

            if (
              id.includes('/form/') ||
              id.includes('/input/') ||
              id.includes('/input-number/') ||
              id.includes('/dynamic-input/') ||
              id.includes('/select/') ||
              id.includes('/radio/') ||
              id.includes('/switch/')
            ) {
              return 'naive-form';
            }

            if (
              id.includes('/button/') ||
              id.includes('/button-group/') ||
              id.includes('/tag/') ||
              id.includes('/empty/') ||
              id.includes('/spin/') ||
              id.includes('/alert/')
            ) {
              return 'naive-basic';
            }

            if (id.includes('/collapse/') || id.includes('/collapse-transition/')) {
              return 'naive-collapse';
            }

            if (id.includes('/dropdown/') || id.includes('/popover/') || id.includes('/tooltip/')) {
              return 'naive-overlay';
            }

            return 'naive-misc';
          }

          if (id.includes('/vueuc/') || id.includes('/vooks/')) {
            return 'naive-shared';
          }

          if (id.includes('/vue/') || id.includes('/pinia/') || id.includes('/vue-i18n/')) {
            return 'vue-vendor';
          }

          if (id.includes('/@tauri-apps/')) {
            return 'tauri';
          }

          if (id.includes('/@lucide/')) {
            return 'icons';
          }

          return 'vendor';
        },
      },
    },
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
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}));
