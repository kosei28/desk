import pandacss from "@pandacss/vite"
import solid from "@solidjs/vite-plugin"
import { defineConfig } from "vitest/config"

export default defineConfig({
  plugins: [pandacss(), solid()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "safari18",
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
  },
})
