/**
 * コードブロックの記法のうち、GitHub が特別に描くものを変換する remark プラグイン。
 * - ```` ```math ````: 数式ブロックにする（rehype-katex が描く。`$$…$$` と同じ）。
 * - ```` ```mermaid ````: `<pre class="mermaid">` にする（ブラウザ側で図にする。scripts/mermaid.ts）。
 *   図の元の文章はそのまま本文に残るので、スクリプトが動かなくても読める。
 */
import type { Root } from "mdast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";

const remarkFences: Plugin<[], Root> = () => (tree) => {
  visit(tree, "code", (node, index, parent) => {
    if (!parent || index === undefined) return;
    if (node.lang === "math") {
      // remark-math が `$$…$$` に付けるものと同じ形にする。
      parent.children[index] = {
        type: "math",
        value: node.value,
        data: {
          hName: "pre",
          hChildren: [
            {
              type: "element",
              tagName: "code",
              properties: { className: ["language-math", "math-display"] },
              children: [{ type: "text", value: node.value }],
            },
          ],
        },
      } as never;
    } else if (node.lang === "mermaid") {
      parent.children[index] = {
        type: "paragraph",
        data: { hName: "pre", hProperties: { className: ["mermaid"] } },
        children: [{ type: "text", value: node.value }],
      };
    }
  });
};

export default remarkFences;
