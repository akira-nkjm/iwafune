/**
 * 表を横スクロールできる枠（`div.table-wrap`）で包む。
 * 列の多い表がページの幅を超えても、ページ全体ではなく表だけが横に動く。
 */
interface HastNode {
  type: string;
  tagName?: string;
  properties?: Record<string, unknown>;
  children?: HastNode[];
}

/** `node` の子孫の `table` を枠で包む（包んだ表の中は見ない）。 */
export function wrapTables(node: HastNode): void {
  if (!node.children) return;
  node.children = node.children.map((child) => {
    if (child.type === "element" && child.tagName === "table") {
      return {
        type: "element",
        tagName: "div",
        properties: { className: ["table-wrap"] },
        children: [child],
      };
    }
    wrapTables(child);
    return child;
  });
}

export default function rehypeTableWrap() {
  return (tree: HastNode) => wrapTables(tree);
}
