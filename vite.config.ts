import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Don't let Vite's file watcher follow the Rust build output.
    // On Windows the compiled vaultotp.exe is locked while running, and
    // chokidar's fs.watch crashes with EBUSY, killing `tauri dev`.
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "node",
  },
});
