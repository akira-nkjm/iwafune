import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";

// reactRouter() プラグインは Vitest 下で動かさないので、vite.config.ts とは別に持つ
export default defineConfig({
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
