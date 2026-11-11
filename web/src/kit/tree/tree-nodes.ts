import type { CatalogItemView, PartRow } from "@/api/types";

/** 表示専用の階層。item が無い節はフォルダー。 */
export type TreeNode<T> = {
  key: string;
  label: string;
  item?: T;
  children: TreeNode<T>[];
};

function folder<T>(roots: TreeNode<T>[], path: string[]): TreeNode<T>[] {
  let children = roots;
  let key = "folder";
  for (const label of path) {
    key += `/${label}`;
    let node = children.find((n) => n.key === key);
    if (!node) {
      node = { key, label, children: [] };
      children.push(node);
    }
    children = node.children;
  }
  return children;
}

/**
 * ファイルの階層（API の `dir`。区間は製品ツリーのノードの ID）を作り、組立品の子は親部品の下に置く。循環は階層にしない。
 * `parts/` の直下の部品はサブシステム（`subsystem`）の節に置く。パスの文字列は分解しない。
 */
export function partsTree<T extends Pick<PartRow, "id" | "parent" | "dir" | "subsystem">>(
  parts: readonly T[],
): TreeNode<T>[] {
  const roots: TreeNode<T>[] = [];
  const nodes = new Map(
    parts.map((p) => [
      p.id,
      { key: `item/${p.id}`, label: p.id, item: p, children: [] } as TreeNode<T>,
    ]),
  );
  for (const part of parts) {
    const node = nodes.get(part.id)!;
    const seen = new Set([part.id]);
    let parent = part.parent;
    while (parent && nodes.has(parent) && !seen.has(parent)) {
      seen.add(parent);
      parent = nodes.get(parent)?.item?.parent ?? null;
    }
    if (part.parent && nodes.has(part.parent) && !(parent && seen.has(parent))) {
      nodes.get(part.parent)!.children.push(node);
    } else {
      const path = part.dir.length ? part.dir : part.subsystem ? [part.subsystem] : [];
      folder(roots, path).push(node);
    }
  }
  return roots;
}

/** カタログ名（API の `scope`）ごとにまとめる。カタログ名の無い品は kind でまとめる。ID は分解しない。 */
export function catalogTree<T extends Pick<CatalogItemView, "id" | "scope" | "name" | "kind">>(
  items: readonly T[],
): TreeNode<T>[] {
  const roots: TreeNode<T>[] = [];
  for (const item of items) {
    folder(roots, [item.scope || item.kind]).push({
      key: `item/${item.id}`,
      label: item.name || item.id,
      item,
      children: [],
    });
  }
  return roots;
}

/** 一致する項目と祖先を残し、結果までの経路を自動展開する。 */
export function filterTree<T>(nodes: readonly TreeNode<T>[], matches: (item: T) => boolean) {
  const expanded = new Set<string>();
  function visit(branch: readonly TreeNode<T>[]): TreeNode<T>[] {
    return branch.flatMap((node) => {
      const children = visit(node.children);
      if (!(node.item && matches(node.item)) && children.length === 0) return [];
      if (children.length) expanded.add(node.key);
      return [{ ...node, children }];
    });
  }
  return { nodes: visit(nodes), expanded };
}

/** フォルダーの件数は子孫の項目数。 */
export function treeCount<T>(node: TreeNode<T>): number {
  return (node.item ? 1 : 0) + node.children.reduce((sum, child) => sum + treeCount(child), 0);
}
