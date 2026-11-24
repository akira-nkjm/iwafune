import type { TreeNode } from "@/api/types";

/** 一覧のカードに出すノードの集まり（製品ツリーのノードと、絞り込みで出すかどうか） */
export interface ShownTree {
  nodes: ReadonlyMap<string, TreeNode>;
  shown: (id: string) => boolean;
}

/** 出している子のノード */
function shownChildren(n: TreeNode, tree: ShownTree): TreeNode[] {
  return n.children
    .map((id) => tree.nodes.get(id))
    .filter((k): k is TreeNode => !!k && tree.shown(k.id));
}

/** ノードの下にある、出している部品の ID（組立品とその子、入れ子のグループの中も含む） */
export function partsWithin(n: TreeNode, tree: ShownTree): string[] {
  return shownChildren(n, tree).flatMap((k) => [
    ...(k.node === "part" ? [k.id] : []),
    ...partsWithin(k, tree),
  ]);
}

/** たためるグループ: 系の下の入れ子の Group で、出すものを中に持つもの（中に組立品や別のグループがあってもよい） */
export function foldable(n: TreeNode, tree: ShownTree): boolean {
  return n.node === "group" && n.parent !== null && shownChildren(n, tree).length > 0;
}

/** ふだんたたんでおくグループ: 中が（子を持たない）部品だけのもの。組立品や別のグループを持つものは、ふだんは開いておく */
export function foldedByDefault(n: TreeNode, tree: ShownTree): boolean {
  return shownChildren(n, tree).every((k) => k.node === "part" && k.children.length === 0);
}
