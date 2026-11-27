/**
 * docs/ の Markdown の相対リンクをサイトのルートか GitHub のリンクに書き換える remark プラグイン。
 * 書き換えの規則は lib/routes.ts の `rewriteLink`（ここは木をたどって当てるだけ）。
 */
import path from "node:path";
import type { Root } from "mdast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";
import { rewriteLink } from "../lib/routes.ts";
import { BASE, docsDir, repoRoot } from "../lib/site.ts";

export interface DocsLinksOptions {
  /** docs/ の場所。省略時は `docsDir()`。 */
  docsDir?: string;
  /** サイトの base。省略時は `BASE`。 */
  base?: string;
}

/** HTML の `<a href="…">` の href を書き換える（Markdown に直接書いた HTML 用）。 */
function rewriteHtmlHrefs(html: string, rewrite: (url: string) => string): string {
  return html.replace(/(<a\s[^>]*?href=)(["'])(.*?)\2/gi, (_m, head, q, url) => {
    return `${head}${q}${rewrite(url)}${q}`;
  });
}

const remarkDocsLinks: Plugin<[DocsLinksOptions?], Root> = (options = {}) => {
  const docs = options.docsDir ?? docsDir();
  const base = options.base ?? BASE;
  return (tree, file) => {
    if (!file.path) return;
    const fileRel = path.relative(repoRoot(docs), file.path).split(path.sep).join("/");
    const rewrite = (url: string) => rewriteLink(url, { fileRel, base });
    visit(tree, (node) => {
      if (node.type === "link" || node.type === "definition") {
        node.url = rewrite(node.url);
      } else if (node.type === "html") {
        node.value = rewriteHtmlHrefs(node.value, rewrite);
      }
    });
  };
};

export default remarkDocsLinks;
