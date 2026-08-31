import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import VueI18nPlugin from "@intlify/unplugin-vue-i18n/vite";
import { fileURLToPath } from "node:url";

// @ts-expect-error process is a nodejs global
const tauriDevHost = process.env.TAURI_DEV_HOST;
const host = tauriDevHost || "127.0.0.1";

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), VueI18nPlugin({
    include: fileURLToPath(new URL("./src/i18n/locales/**", import.meta.url)),
    strictMessage: false, // Technical text such as <seg> is rendered as escaped text, never HTML.
  })],
  define: {
    __VUE_I18N_LEGACY_API__: false,
    __INTLIFY_PROD_DEVTOOLS__: false,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host,
    hmr: tauriDevHost
      ? {
          protocol: "ws",
          host: tauriDevHost,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
