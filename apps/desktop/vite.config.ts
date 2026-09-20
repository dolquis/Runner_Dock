import react from "@vitejs/plugin-react";
// vitest の設定も同じファイルで持つため vitest/config から取り込む。
import { defineConfig } from "vitest/config";

// Tauri の dev サーバー設定。ポートは tauri.conf.json の devUrl と揃える。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    // ローカル UI のみを対象にする。外部公開しない。
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "es2022",
    sourcemap: true,
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
