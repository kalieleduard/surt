import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  test: {
    include: ["src/**/*.test.ts"],
    coverage: {
      provider: "v8",
      include: ["src/engine/*.ts", "src/ui/time.ts"],
      exclude: ["src/**/*.test.ts", "src/engine/fixtures.ts", "src/engine/types.ts", "src/engine/client.ts", "src/engine/usePolled.ts", "src/engine/useAction.ts"],
      thresholds: { lines: 90 },
    },
  },
});
