/**
 * ページの題名は front matter の `title`（loaders/docs.ts が最初の `# H1` から作る）で出るので、
 * 本文の先頭の `# H1` を取り除いて二重に出ないようにする remark プラグイン。
 */
import type { Root } from "mdast";
import type { Plugin } from "unified";

const remarkDocsTitle: Plugin<[], Root> = () => (tree) => {
  const index = tree.children.findIndex((n) => n.type === "heading" && n.depth === 1);
  if (index >= 0) tree.children.splice(index, 1);
};

export default remarkDocsTitle;
