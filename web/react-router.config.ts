import { cp, rm } from "node:fs/promises";
import { join } from "node:path";
import type { Config } from "@react-router/dev/config";

// SPA モード: サーバーレンダリングなし。ビルド時に root の Layout + HydrateFallback を index.html として書き出す。
// fune-server は web/dist をそのまま埋め込んで配信するので、client の出力を dist 直下へ移す。
export default {
  appDirectory: "src",
  ssr: false,
  buildDirectory: "dist",
  async buildEnd({ reactRouterConfig }) {
    const dir = reactRouterConfig.buildDirectory;
    await rm(join(dir, "server"), { recursive: true, force: true });
    await cp(join(dir, "client"), dir, { recursive: true });
    await rm(join(dir, "client"), { recursive: true, force: true });
  },
} satisfies Config;
