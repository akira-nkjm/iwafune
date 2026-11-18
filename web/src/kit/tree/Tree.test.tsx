import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { Tree } from "./Tree";
import type { TreeNode } from "./tree-nodes";

type Item = { id: string };
const leaf = (id: string): TreeNode<Item> => ({
  key: `item/${id}`,
  label: id,
  item: { id },
  children: [],
});
const render = (nodes: TreeNode<Item>[]) =>
  renderToStaticMarkup(
    <Tree
      nodes={nodes}
      selected={null}
      onSelect={() => {}}
      label="t"
      renderItem={(_, label) => <span>{label}</span>}
    />,
  );

describe("Tree toolbar", () => {
  const t = getT().common;
  it("shows expand-all and collapse-all buttons with names and titles when there are folders", () => {
    const html = render([{ key: "folder/a", label: "a", children: [leaf("X")] }]);
    expect(html).toContain(`aria-label="${t.expandAll}"`);
    expect(html).toContain(`title="${t.collapseAll}"`);
    // 初期状態はすべて開いているので「開く」は無効、「閉じる」は有効
    const button = (name: string) =>
      html.match(new RegExp(`<button[^>]*aria-label="${name}"[^>]*>`))?.[0];
    expect(button(t.expandAll)).toContain(' disabled=""');
    expect(button(t.collapseAll)).not.toContain(' disabled=""');
  });
  it("has no buttons when there are no folders", () => {
    const html = render([leaf("X"), leaf("Y")]);
    expect(html).not.toContain(t.expandAll);
    expect(html).not.toContain(t.collapseAll);
  });
});
