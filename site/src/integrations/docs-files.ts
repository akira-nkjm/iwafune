/**
 * docs/ の画像（図の SVG など）をサイトの `<base>/docs-files/…` で配る Astro の統合。
 * 開発サーバーでは docs/ から直接返し、ビルドでは dist/docs-files/ にコピーする（docs/ にコピーを置かない）。
 */
import fs from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { AstroIntegration } from "astro";
import { DOCS_FILES_ROUTE, IMAGE_EXTENSIONS } from "../lib/routes.ts";
import { BASE, docsDir } from "../lib/site.ts";

/** docs/ の下の画像ファイル（docs/ からの相対パス、`/` 区切り）。 */
export function listDocsImages(docs: string): string[] {
  if (!fs.existsSync(docs)) return [];
  return (fs.readdirSync(docs, { recursive: true, encoding: "utf-8" }) as string[])
    .map((f) => f.split(path.sep).join("/"))
    .filter((f) => IMAGE_EXTENSIONS.includes(path.extname(f).toLowerCase()))
    .sort();
}

const MIME: Record<string, string> = {
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".avif": "image/avif",
};

export default function docsFiles(): AstroIntegration {
  const docs = docsDir();
  return {
    name: "iwafune-docs-files",
    hooks: {
      "astro:server:setup": ({ server }) => {
        // Astro が base を外してから渡すことも、付けたまま渡すこともあるので、両方の形を受ける。
        const prefixes = [`${BASE}${DOCS_FILES_ROUTE}`, DOCS_FILES_ROUTE];
        const handle = (req: IncomingMessage, res: ServerResponse, next: () => void) => {
          const url = (req.url ?? "").split("?")[0] ?? "";
          const prefix = prefixes.find((p) => url.startsWith(p));
          if (!prefix) return next();
          const rel = decodeURIComponent(url.slice(prefix.length));
          const file = path.resolve(docs, rel);
          const inside = file.startsWith(docs + path.sep);
          if (!inside || !fs.existsSync(file) || !fs.statSync(file).isFile()) return next();
          res.setHeader(
            "Content-Type",
            MIME[path.extname(file).toLowerCase()] ?? "application/octet-stream",
          );
          fs.createReadStream(file).pipe(res);
        };
        // Astro 自身のルーティング（無いページに 404 を返す）より前に置く。後ろに足すと、図の要求が届かない。
        server.middlewares.stack.unshift({ route: "", handle });
      },
      "astro:build:done": ({ dir, logger }) => {
        const out = path.join(fileURLToPath(dir), DOCS_FILES_ROUTE);
        const files = listDocsImages(docs);
        for (const rel of files) {
          fs.mkdirSync(path.dirname(path.join(out, rel)), { recursive: true });
          fs.copyFileSync(path.join(docs, rel), path.join(out, rel));
        }
        logger.info(`copied ${files.length} image(s) from docs/ to ${DOCS_FILES_ROUTE}`);
      },
    },
  };
}
