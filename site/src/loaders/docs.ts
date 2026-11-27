/**
 * docs/ の Markdown を Starlight の docs コレクションとして読むローダー。
 * Astro 標準の `glob` ローダーを包み、front matter のないページ（docs の書き方）にも題名・説明・編集リンクを付ける:
 * - `title`: 最初の `# H1`（本文からは remark-docs-title が取り除く）。
 * - `description`: H1 のあとの最初の段落。
 * - `editUrl`: GitHub の `develop` ブランチのそのファイル。
 * front matter に書いてあるものは、それを優先する。
 */
import fs from "node:fs/promises";
import path from "node:path";
import type { Loader } from "astro/loaders";
import { glob } from "astro/loaders";
import { deriveMeta } from "../lib/docs-meta.ts";
import { docsDir, EDIT_BRANCH, REPO_URL, repoRoot } from "../lib/site.ts";

/** docs/ の Markdown をすべて読む（`_` で始まるファイルは除く）。 */
export function docsLoader(): Loader {
  const base = docsDir();
  return {
    name: "iwafune-docs-loader",
    load: async (context) => {
      const inner = glob({ base, pattern: "**/[^_]*.md" });
      await inner.load({
        ...context,
        parseData: async (props) => {
          const text = await fs.readFile(props.filePath ?? "", "utf-8").catch(() => "");
          const meta = deriveMeta(text);
          const rel = path
            .relative(repoRoot(base), props.filePath ?? "")
            .split(path.sep)
            .join("/");
          const data = {
            ...(meta.title ? { title: meta.title } : {}),
            ...(meta.description ? { description: meta.description } : {}),
            editUrl: `${REPO_URL}/edit/${EDIT_BRANCH}/${rel}`,
            ...(props.data as Record<string, unknown>),
          };
          return context.parseData({ ...props, data: data as unknown as typeof props.data });
        },
      });
    },
  };
}
