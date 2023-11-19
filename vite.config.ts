import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import path from "path";
import Icons from "unplugin-icons/vite";
import Components from "unplugin-vue-components/vite";
import IconsResolver from "unplugin-icons/resolver";

const mobile =
  process.env.TAURI_PLATFORM === "android" ||
  process.env.TAURI_PLATFORM === "ios";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    Components({
      dts: true,
      resolvers: [IconsResolver()],
    }),
    Icons(),
  ],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
  }
}));
