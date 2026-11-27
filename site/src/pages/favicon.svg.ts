/**
 * ファビコンは GUI のもの（web/public/favicon.svg）をそのまま返す（コピーを置かない）。
 */
import fs from "node:fs";
import path from "node:path";
import { SITE_ROOT } from "../lib/site.ts";

export const GET = () => {
  const file = path.resolve(SITE_ROOT, "..", "web", "public", "favicon.svg");
  return new Response(fs.readFileSync(file), { headers: { "Content-Type": "image/svg+xml" } });
};
