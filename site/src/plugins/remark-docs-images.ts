/**
 * docs/ の Markdown の画像（`![…](../assets/figures/x.svg)`）を、サイトが配るファイルの URL に書き換える remark プラグイン。
 * Astro の画像処理は通さない（docs/ の図は SVG が多く、そのまま配るほうが確かなため）。書き換えの規則は lib/routes.ts の
 * `rewriteImage`。配るのは integrations/docs-files.ts。
 */
import path from "node:path";
import type { Root } from "mdast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";
import { rewriteImage } from "../lib/routes.ts";
import { BASE, docsDir, repoRoot } from "../lib/site.ts";
import type { DocsLinksOptions } from "./remark-docs-links.ts";

/** HTML の `<img src="…">` の src を書き換える。 */
function rewriteHtmlSrcs(html: string, rewrite: (url: string) => string): string {
  return html.replace(/(<img\s[^>]*?src=)(["'])(.*?)\2/gi, (_m, head, q, url) => {
    return `${head}${q}${rewrite(url)}${q}`;
  });
}

const remarkDocsImages: Plugin<[DocsLinksOptions?], Root> = (options = {}) => {
  const docs = options.docsDir ?? docsDir();
  const base = options.base ?? BASE;
  return (tree, file) => {
    if (!file.path) return;
    const fileRel = path.relative(repoRoot(docs), file.path).split(path.sep).join("/");
    const rewrite = (url: string) => rewriteImage(url, { fileRel, base });
    visit(tree, (node) => {
      if (node.type === "image") {
        node.url = rewrite(node.url);
      } else if (node.type === "html") {
        node.value = rewriteHtmlSrcs(node.value, rewrite);
      }
    });
  };
};

export default remarkDocsImages;
