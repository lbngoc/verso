import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

// Multi-page build: one HTML entry per Tauri window (popup, quick, settings, onboarding).
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: resolve(__dirname, "src/lib"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/specs/**", "**/references/**", "**/spikes/**"] },
  },
  build: {
    target: "safari15",
    minify: "esbuild",
    sourcemap: false,
    rollupOptions: {
      input: {
        popup: resolve(__dirname, "popup.html"),
        quick: resolve(__dirname, "quick.html"),
        settings: resolve(__dirname, "settings.html"),
        onboarding: resolve(__dirname, "onboarding.html"),
      },
    },
  },
});
